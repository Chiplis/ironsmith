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
use crate::cost::{OptionalCostsPaid, PermanentFilter};
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
    CreatureAttackedEvent, CreatureBecameBlockedEvent, CreatureBlockedEvent,
};
use crate::events::damage::DamageEvent;
use crate::events::life::LifeLossEvent;
use crate::events::permanents::SacrificeEvent;
use crate::events::spells::{AbilityActivatedEvent, BecomesTargetedEvent, SpellCastEvent};
use crate::events::zones::EnterBattlefieldEvent;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
use crate::game_event::DamageTarget as EventDamageTarget;
use crate::game_state::{GameState, Phase, StackEntry, Step, Target};
use crate::ids::{ObjectId, PlayerId, StableId};
#[cfg(feature = "net")]
use crate::net::{CostPayment, CostStep, GameObjectId, ManaSymbolCode, ManaSymbolSpec, ZoneCode};
#[cfg(not(feature = "net"))]
type CostStep = ();
use crate::object::CounterType;
use crate::player::ManaPool;
use crate::rules::combat::{
    deals_first_strike_damage_with_game, deals_regular_combat_damage_with_game, maximum_blockers,
    minimum_blockers,
};
use crate::rules::damage::{
    DamageResult, DamageTarget, calculate_damage_with_game, distribute_trample_damage,
};
use crate::rules::state_based::{apply_state_based_actions_with, check_state_based_actions};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::triggers::{
    DamageEventTarget, TriggerEvent, TriggerQueue, TriggeredAbilityEntry, check_triggers,
    generate_step_trigger_events, verify_intervening_if,
};
use crate::turn::{
    PriorityResult, PriorityTracker, TurnError, execute_cleanup_step, execute_draw_step,
    execute_untap_step, pass_priority, reset_priority,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during game loop execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameLoopError {
    /// Turn progression error.
    TurnError(TurnError),
    /// Stack resolution failed.
    ResolutionFailed(String),
    /// Invalid game state.
    InvalidState(String),
    /// No players remaining.
    GameOver,
    /// Invalid player response.
    ResponseError(ResponseError),
    /// Combat error.
    CombatError(CombatError),
    /// Special action error.
    ActionError(crate::special_actions::ActionError),
}

/// Response payload for externally driving a pending priority decision.
///
/// This is intentionally limited to decisions that can occur during the
/// priority loop (`GameProgress::NeedsDecisionCtx`).
#[derive(Debug, Clone, PartialEq)]
pub enum PriorityResponse {
    PriorityAction(LegalAction),
    Attackers(Vec<AttackerDeclaration>),
    Blockers {
        defending_player: PlayerId,
        declarations: Vec<BlockerDeclaration>,
    },
    Targets(Vec<Target>),
    XValue(u32),
    NumberChoice(u32),
    Modes(Vec<usize>),
    OptionalCosts(Vec<(usize, u32)>),
    ManaPayment(usize),
    ManaPipPayment(usize),
    SacrificeTarget(ObjectId),
    CardToExile(ObjectId),
    HybridChoice(usize),
    CastingMethodChoice(usize),
    ReplacementChoice(usize),
}

impl From<TurnError> for GameLoopError {
    fn from(err: TurnError) -> Self {
        GameLoopError::TurnError(err)
    }
}

impl From<ResponseError> for GameLoopError {
    fn from(err: ResponseError) -> Self {
        GameLoopError::ResponseError(err)
    }
}

impl From<CombatError> for GameLoopError {
    fn from(err: CombatError) -> Self {
        GameLoopError::CombatError(err)
    }
}

impl From<crate::special_actions::ActionError> for GameLoopError {
    fn from(err: crate::special_actions::ActionError) -> Self {
        GameLoopError::ActionError(err)
    }
}

impl std::fmt::Display for GameLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameLoopError::TurnError(e) => write!(f, "Turn error: {:?}", e),
            GameLoopError::ResolutionFailed(msg) => write!(f, "Resolution failed: {}", msg),
            GameLoopError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            GameLoopError::GameOver => write!(f, "Game over"),
            GameLoopError::ResponseError(e) => write!(f, "Response error: {}", e),
            GameLoopError::CombatError(e) => write!(f, "Combat error: {}", e),
            GameLoopError::ActionError(e) => write!(f, "Action error: {:?}", e),
        }
    }
}

impl std::error::Error for GameLoopError {}

// ============================================================================
// Target Extraction
// ============================================================================

/// Check if a ChooseSpec requires player selection.
/// Check if a target spec requires the player to select a target.
pub fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        // Target wrapper - check the inner spec
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            requires_target_selection(inner)
        }
        // These require target selection during casting
        ChooseSpec::AnyTarget
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::Player(_)
        | ChooseSpec::Object(_) => true,
        ChooseSpec::AttackedPlayerOrPlaneswalker => false,
        // These don't require selection - they're resolved at execution time
        _ => false,
    }
}

/// Queue trigger matches for all triggered abilities that see this event.
fn queue_triggers_for_event(
    game: &GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
) {
    let triggers = check_triggers(game, &event);
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
}

/// Ingest an event into trigger system with optional delayed-trigger checks.
fn queue_triggers_from_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
    include_delayed: bool,
) {
    if let Some(damage_event) = event.downcast::<DamageEvent>()
        && let EventDamageTarget::Player(player_id) = damage_event.target
    {
        *game
            .damage_to_players_this_turn
            .entry(player_id)
            .or_insert(0) += damage_event.amount;

        if game
            .object(damage_event.source)
            .map(|o| o.is_creature())
            .unwrap_or(false)
        {
            *game
                .creature_damage_to_players_this_turn
                .entry(player_id)
                .or_insert(0) += damage_event.amount;
        }
    }

    game.record_trigger_event_kind(event.kind());
    queue_triggers_for_event(game, trigger_queue, event.clone());

    if include_delayed {
        let delayed = crate::triggers::check_delayed_triggers(game, &event);
        for trigger in delayed {
            trigger_queue.add(trigger);
        }
    }
}

/// Queue trigger matches for each event in this list.
fn queue_triggers_for_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) {
    for event in events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
}

fn target_events_from_targets(
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
) -> Vec<TriggerEvent> {
    targets
        .iter()
        .filter_map(|target| {
            let Target::Object(target_id) = target else {
                return None;
            };
            Some(TriggerEvent::new(BecomesTargetedEvent::new(
                *target_id,
                source,
                source_controller,
                by_ability,
            )))
        })
        .collect()
}

fn queue_becomes_targeted_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
) {
    for event in target_events_from_targets(targets, source, source_controller, by_ability) {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }
}

fn queue_ability_activated_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    source: ObjectId,
    activator: PlayerId,
    is_mana_ability: bool,
    source_stable_id: Option<StableId>,
) {
    let snapshot = if let Some(obj) = game.object(source) {
        Some(ObjectSnapshot::from_object(obj, game))
    } else if let Some(stable_id) = source_stable_id {
        game.find_object_by_stable_id(stable_id)
            .and_then(|id| game.object(id))
            .map(|obj| ObjectSnapshot::from_object(obj, game))
    } else {
        None
    };
    if is_mana_ability {
        let is_land_source = game
            .object(source)
            .map(|obj| obj.is_land())
            .or_else(|| snapshot.as_ref().map(|snap| snap.is_land()))
            .unwrap_or(false);
        if is_land_source {
            game.players_tapped_land_for_mana_this_turn
                .insert(activator);
        }
    }
    let event = TriggerEvent::new(
        AbilityActivatedEvent::new(source, activator, is_mana_ability).with_snapshot(snapshot),
    );
    queue_triggers_from_event(game, trigger_queue, event, true);
}

fn queue_mana_ability_event_for_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    action: &ManaPipPaymentAction,
    activator: PlayerId,
) {
    if let ManaPipPaymentAction::ActivateManaAbility { source_id, .. } = action {
        queue_ability_activated_event(game, trigger_queue, *source_id, activator, true, None);
    }
}

fn tap_permanent_with_trigger(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    permanent: ObjectId,
) {
    if game.object(permanent).is_some() && !game.is_tapped(permanent) {
        game.tap(permanent);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new(crate::events::PermanentTappedEvent::new(permanent)),
            true,
        );
    }
}

fn keyword_action_from_alternative_effect(effect: AlternativePaymentEffect) -> KeywordActionKind {
    match effect {
        AlternativePaymentEffect::Convoke => KeywordActionKind::Convoke,
        AlternativePaymentEffect::Improvise => KeywordActionKind::Improvise,
    }
}

fn payment_contribution_tag(effect: AlternativePaymentEffect) -> &'static str {
    match effect {
        AlternativePaymentEffect::Convoke => "convoked_this_spell",
        AlternativePaymentEffect::Improvise => "improvised_this_spell",
    }
}

fn record_keyword_payment_contribution(
    contributions: &mut Vec<KeywordPaymentContribution>,
    action: &ManaPipPaymentAction,
) {
    let ManaPipPaymentAction::PayViaAlternative {
        permanent_id,
        effect,
    } = action
    else {
        return;
    };

    let contribution = KeywordPaymentContribution {
        permanent_id: *permanent_id,
        effect: *effect,
    };
    if !contributions.contains(&contribution) {
        contributions.push(contribution);
    }
}

fn apply_keyword_payment_tags_for_resolution(
    game: &GameState,
    entry: &StackEntry,
    ctx: &mut ExecutionContext,
) {
    for contribution in &entry.keyword_payment_contributions {
        if let Some(obj) = game.object(contribution.permanent_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object(payment_contribution_tag(contribution.effect), snapshot);
        }
    }
}

/// Drain pending death and custom trigger events and enqueue all matches.
fn drain_pending_trigger_events(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    let pending_events = game.take_pending_trigger_events();
    for event in pending_events {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }
}

/// Extracted target information from an effect.
pub struct ExtractedTarget<'a> {
    pub spec: &'a ChooseSpec,
    pub description: &'static str,
    pub min_targets: usize,
    pub max_targets: Option<usize>,
}

/// Extract a ChooseSpec from an Effect, if it has one that requires selection.
pub fn extract_target_spec(effect: &Effect) -> Option<ExtractedTarget<'_>> {
    // Use the EffectExecutor trait methods on the wrapped executor
    effect.0.get_target_spec().map(|spec| {
        // Check if the effect has a target count override
        let (min_targets, max_targets) = if let Some(target_count) = effect.0.get_target_count() {
            (target_count.min, target_count.max)
        } else {
            // Default: exactly 1 target
            (1, Some(1))
        };

        ExtractedTarget {
            spec,
            description: effect.0.target_description(),
            min_targets,
            max_targets,
        }
    })
}

/// Extract target requirements from a list of effects.
fn extract_target_requirements(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<TargetRequirement> {
    let mut requirements = Vec::new();

    for effect in effects {
        if let Some(extracted) = extract_target_spec(effect)
            && requires_target_selection(extracted.spec)
        {
            let legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
            // For "any number" effects (min_targets == 0), we can cast even with no legal targets.
            // For required targets (min_targets > 0), we need at least min_targets legal targets.
            let has_enough_targets =
                extracted.min_targets == 0 || legal_targets.len() >= extracted.min_targets;
            if has_enough_targets {
                requirements.push(TargetRequirement {
                    spec: extracted.spec.clone(),
                    legal_targets,
                    description: extracted.description.to_string(),
                    min_targets: extracted.min_targets,
                    max_targets: extracted.max_targets,
                });
            }
        }
    }

    requirements
}

/// Check if a spell has all required legal targets.
/// Returns true if all targeting requirements have enough legal targets,
/// or if the spell has no targeting requirements.
/// For "any number" effects (min_targets == 0), no legal targets are required.
pub fn spell_has_legal_targets(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> bool {
    for effect in effects {
        if let Some(extracted) = extract_target_spec(effect)
            && requires_target_selection(extracted.spec)
        {
            // For "any number" effects, we can cast even with no legal targets
            if extracted.min_targets == 0 {
                continue;
            }
            let legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
            if legal_targets.len() < extracted.min_targets {
                return false;
            }
        }
    }
    true
}

/// Compute legal targets for a given ChooseSpec.
///
/// The `caster` parameter is used for resolving "you control" and similar filters.
/// The `source_id` is used for "other" filters (exclude the source itself).
pub fn compute_legal_targets(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    match spec {
        ChooseSpec::AnyTarget => {
            let mut targets = Vec::new();
            // All players
            for player in &game.players {
                if player.is_in_game() {
                    targets.push(Target::Player(player.id));
                }
            }
            // All creatures and planeswalkers on battlefield
            for &obj_id in &game.battlefield {
                if let Some(obj) = game.object(obj_id)
                    && (obj.has_card_type(crate::types::CardType::Creature)
                        || obj.has_card_type(crate::types::CardType::Planeswalker))
                {
                    // Check hexproof/shroud - can't target if controlled by opponent
                    // and has cant_be_targeted flag
                    let is_untargetable = game.is_untargetable(obj_id);
                    let is_controlled_by_caster = obj.controller == caster;
                    if is_untargetable && !is_controlled_by_caster {
                        continue;
                    }

                    targets.push(Target::Object(obj_id));
                }
            }
            targets
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            let mut targets = Vec::new();
            for player in &game.players {
                if player.is_in_game() && player_matches_filter(player.id, filter, game, caster) {
                    targets.push(Target::Player(player.id));
                }
            }
            for &obj_id in &game.battlefield {
                if let Some(obj) = game.object(obj_id)
                    && obj.has_card_type(crate::types::CardType::Planeswalker)
                {
                    let is_untargetable = game.is_untargetable(obj_id);
                    let is_controlled_by_caster = obj.controller == caster;
                    if is_untargetable && !is_controlled_by_caster {
                        continue;
                    }
                    targets.push(Target::Object(obj_id));
                }
            }
            targets
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => Vec::new(),
        ChooseSpec::Player(filter) => {
            let mut targets = Vec::new();
            for player in &game.players {
                if player.is_in_game() && player_matches_filter(player.id, filter, game, caster) {
                    targets.push(Target::Player(player.id));
                }
            }
            targets
        }
        ChooseSpec::Object(filter) => {
            let mut targets = Vec::new();
            // Check battlefield objects
            for &obj_id in &game.battlefield {
                if let Some(obj) = game.object(obj_id)
                    && object_matches_filter(obj, filter, game, caster, source_id)
                {
                    // Check if the object can be targeted (hexproof/shroud)
                    // Hexproof only prevents opponents from targeting
                    // Shroud prevents everyone from targeting
                    let is_untargetable = game.is_untargetable(obj_id);
                    let is_controlled_by_caster = obj.controller == caster;

                    // If the object has shroud (can't be targeted by anyone), skip it
                    // If the object has hexproof (tracked in cant_be_targeted) and
                    // is controlled by an opponent, skip it
                    // Note: This is simplified - full implementation would distinguish
                    // hexproof (opponents only) from shroud (everyone)
                    if is_untargetable && !is_controlled_by_caster {
                        continue;
                    }

                    // Check if the target has protection from the source
                    if let Some(source_id) = source_id
                        && has_protection_from_source(game, obj_id, source_id)
                    {
                        continue;
                    }

                    targets.push(Target::Object(obj_id));
                }
            }
            // Check stack for spells (for counterspells)
            if filter.zone == Some(Zone::Stack) || filter.zone.is_none() {
                for entry in &game.stack {
                    if let Some(obj) = game.object(entry.object_id)
                        && object_matches_filter(obj, filter, game, caster, source_id)
                    {
                        // Stack objects (spells) can have "can't be targeted" too
                        // but it's less common
                        targets.push(Target::Object(entry.object_id));
                    }
                }
            }
            // Check graveyard for cards (for Snapcaster Mage, etc.)
            if filter.zone == Some(Zone::Graveyard) || filter.zone.is_none() {
                for player in &game.players {
                    for &obj_id in &player.graveyard {
                        if let Some(obj) = game.object(obj_id)
                            && object_matches_filter(obj, filter, game, caster, source_id)
                        {
                            targets.push(Target::Object(obj_id));
                        }
                    }
                }
            }
            targets
        }
        // Target wrapper - recursively compute targets from inner spec
        ChooseSpec::Target(inner) => compute_legal_targets(game, inner, caster, source_id),
        // WithCount wrapper - recursively compute targets from inner spec
        ChooseSpec::WithCount(inner, _) => compute_legal_targets(game, inner, caster, source_id),
        // These don't require selection - they're resolved at execution time
        ChooseSpec::Source
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::Tagged(_)
        | ChooseSpec::All(_)
        | ChooseSpec::EachPlayer(_)
        | ChooseSpec::Iterated => Vec::new(),
    }
}

/// Check if a player matches a PlayerFilter.
fn player_matches_filter(
    player_id: PlayerId,
    filter: &crate::target::PlayerFilter,
    game: &GameState,
    controller: PlayerId,
) -> bool {
    player_matches_filter_with_combat(player_id, filter, game, controller, None)
}

/// Check if a player matches a PlayerFilter with explicit combat context.
pub fn player_matches_filter_with_combat(
    player_id: PlayerId,
    filter: &crate::target::PlayerFilter,
    game: &GameState,
    controller: PlayerId,
    combat: Option<&CombatState>,
) -> bool {
    use crate::combat_state::{get_attacking_player, is_defending_player};
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::Any => true,
        PlayerFilter::You => player_id == controller,
        PlayerFilter::NotYou => player_id != controller,
        PlayerFilter::Opponent => player_id != controller,
        PlayerFilter::Active => game.turn.active_player == player_id,
        PlayerFilter::Teammate => false, // In 2-player games, no teammates
        PlayerFilter::Defending => combat
            .map(|c| is_defending_player(c, player_id))
            .unwrap_or(false),
        PlayerFilter::Attacking => combat
            .map(|c| get_attacking_player(c, game) == Some(player_id))
            .unwrap_or(false),
        PlayerFilter::DamagedPlayer => false,
        PlayerFilter::Specific(id) => player_id == *id,
        PlayerFilter::IteratedPlayer => {
            // IteratedPlayer is resolved at runtime during iteration, not here
            false
        }
        PlayerFilter::Target(_) => {
            // Target filters are resolved through targeting, not direct matching
            true
        }
        PlayerFilter::ControllerOf(_) | PlayerFilter::OwnerOf(_) => {
            // These require object resolution, not applicable for simple player matching
            false
        }
    }
}

/// Check if an object matches an ObjectFilter using full filter logic.
fn object_matches_filter(
    obj: &crate::object::Object,
    filter: &ObjectFilter,
    game: &GameState,
    controller: PlayerId,
    source_id: Option<ObjectId>,
) -> bool {
    object_matches_filter_with_combat(obj, filter, game, controller, source_id, None)
}

/// Check if an object matches an ObjectFilter using full filter logic, with combat context.
fn object_matches_filter_with_combat(
    obj: &crate::object::Object,
    filter: &ObjectFilter,
    game: &GameState,
    controller: PlayerId,
    source_id: Option<ObjectId>,
    combat: Option<&CombatState>,
) -> bool {
    use crate::combat_state::{defending_players, get_attacking_player};

    // Get combat context if in combat
    let (defending_player, attacking_player) = if let Some(combat) = combat {
        let defenders = defending_players(combat);
        let attacker = get_attacking_player(combat, game);
        // For 2-player games, there's typically one defending player
        (defenders.into_iter().next(), attacker)
    } else {
        (None, None)
    };

    let ctx =
        game.filter_context_for_combat(controller, source_id, defending_player, attacking_player);

    filter.matches(obj, &ctx, game)
}

/// Check if an object has protection from a source that would prevent targeting.
///
/// Protection from a quality prevents:
/// - Damage from sources with that quality
/// - Enchanting/Equipping by permanents with that quality
/// - Blocking by creatures with that quality
/// - Targeting by spells/abilities from sources with that quality
fn has_protection_from_source(game: &GameState, target_id: ObjectId, source_id: ObjectId) -> bool {
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbility;

    let Some(target) = game.object(target_id) else {
        return false;
    };
    let Some(source) = game.object(source_id) else {
        return false;
    };

    // Check target's abilities for protection
    // Use calculated characteristics to account for effects like Humility
    let target_abilities: Vec<StaticAbility> = game
        .calculated_characteristics(target_id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| {
            target
                .abilities
                .iter()
                .filter_map(|a| {
                    if let AbilityKind::Static(sa) = &a.kind {
                        Some(sa.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });

    for ability in target_abilities {
        if ability.has_protection()
            && let Some(protection_from) = ability.protection_from()
            && source_matches_protection(source, protection_from, game)
        {
            return true;
        }
    }

    false
}

/// Check if a source matches a protection quality.
fn source_matches_protection(
    source: &crate::object::Object,
    protection: &crate::ability::ProtectionFrom,
    game: &GameState,
) -> bool {
    use crate::ability::ProtectionFrom;

    match protection {
        ProtectionFrom::Color(colors) => {
            // Check if source has any of the protected colors
            let source_colors = game
                .calculated_characteristics(source.id)
                .map(|c| c.colors)
                .unwrap_or_else(|| source.colors());
            !source_colors.intersection(*colors).is_empty()
        }
        ProtectionFrom::AllColors => {
            // Source must have at least one color
            let source_colors = game
                .calculated_characteristics(source.id)
                .map(|c| c.colors)
                .unwrap_or_else(|| source.colors());
            !source_colors.is_empty()
        }
        ProtectionFrom::Creatures => {
            // Check if source is a creature
            source.has_card_type(crate::types::CardType::Creature)
        }
        ProtectionFrom::CardType(card_type) => {
            // Check if source has the card type
            source.has_card_type(*card_type)
        }
        ProtectionFrom::Permanents(filter) => {
            // Check if source matches the filter
            // Use a context for the filter matching
            let ctx = crate::target::FilterContext::new(source.controller);
            filter.matches(source, &ctx, game)
        }
        ProtectionFrom::Everything => {
            // Protection from everything protects from all sources
            true
        }
        ProtectionFrom::Colorless => {
            // Check if source is colorless (has no colors)
            let source_colors = game
                .calculated_characteristics(source.id)
                .map(|c| c.colors)
                .unwrap_or_else(|| source.colors());
            source_colors.is_empty()
        }
    }
}

/// Validate targets for a stack entry that's about to resolve.
///
/// Per MTG Rule 608.2b:
/// - If a spell/ability has targets and ALL targets are now illegal, it fizzles
/// - If SOME targets are still legal, the spell/ability resolves and does as much as possible
///
/// Returns (valid_targets, all_targets_invalid)
fn validate_stack_entry_targets(
    game: &GameState,
    entry: &StackEntry,
) -> (Vec<ResolvedTarget>, bool) {
    if entry.targets.is_empty() {
        return (Vec::new(), false);
    }

    let aura_target_spec = if let Some(obj) = game.object(entry.object_id)
        && obj.has_card_type(CardType::Enchantment)
        && obj.subtypes.contains(&crate::types::Subtype::Aura)
    {
        let effects = get_effects_for_stack_entry(game, entry, obj);
        effects
            .iter()
            .filter_map(extract_target_spec)
            .map(|extracted| extracted.spec.clone())
            .next()
    } else {
        None
    };

    let aura_ctx =
        crate::executor::ExecutionContext::new_default(entry.object_id, entry.controller);

    let mut valid_targets = Vec::new();
    let mut invalid_count = 0;

    for target in &entry.targets {
        let is_valid = match target {
            Target::Object(obj_id) => {
                // Check if object still exists
                if let Some(obj) = game.object(*obj_id) {
                    // Check if object is still on battlefield or stack (as appropriate)
                    let in_valid_zone = obj.zone == Zone::Battlefield || obj.zone == Zone::Stack;

                    // Check if object now has protection from the source
                    let has_protection = has_protection_from_source(game, *obj_id, entry.object_id);

                    // Check hexproof/shroud
                    let is_untargetable = game.is_untargetable(*obj_id);
                    let source_controller = game.object(entry.object_id).map(|s| s.controller);
                    let target_controller = obj.controller;
                    // Hexproof only prevents opponents from targeting
                    let blocked_by_hexproof = is_untargetable
                        && source_controller.is_some_and(|sc| sc != target_controller);

                    let mut valid = in_valid_zone && !has_protection && !blocked_by_hexproof;

                    if valid && let Some(ref spec) = aura_target_spec {
                        let resolved = crate::executor::ResolvedTarget::Object(*obj_id);
                        valid = crate::effects::helpers::validate_target(
                            game, &resolved, spec, &aura_ctx,
                        );
                    }

                    valid
                } else {
                    // Object no longer exists
                    false
                }
            }
            Target::Player(player_id) => {
                // Check if player is still in the game
                game.player(*player_id)
                    .map(|p| p.is_in_game())
                    .unwrap_or(false)
            }
        };

        if is_valid {
            valid_targets.push(match target {
                Target::Object(id) => ResolvedTarget::Object(*id),
                Target::Player(id) => ResolvedTarget::Player(*id),
            });
        } else {
            invalid_count += 1;
        }
    }

    let all_invalid = invalid_count == entry.targets.len();
    (valid_targets, all_invalid)
}

// ============================================================================
// Stack Resolution
// ============================================================================

/// Resolve the top entry on the stack.
///
/// This function:
/// 1. Pops the top entry from the stack
/// 2. Validates targets
/// 3. Executes effects
/// 4. Moves spell to graveyard (if spell, not ability)
///
/// Note: May effects will be auto-declined. Use `resolve_stack_entry_with` to
/// provide a decision maker for interactive May choices.
pub fn resolve_stack_entry(game: &mut GameState) -> Result<(), GameLoopError> {
    let mut auto_dm = crate::decision::AutoPassDecisionMaker;
    resolve_stack_entry_full(game, &mut auto_dm, None)
}

/// Resolve the top entry on the stack with both a decision maker and trigger queue.
///
/// Use this for ETB replacement effects that need player decisions (like Mox Diamond).
fn resolve_stack_entry_with_dm_and_triggers(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, Some(trigger_queue))
}

/// Resolve the top entry on the stack with an optional decision maker.
///
/// If a decision maker is provided, May effects will prompt the player.
/// Otherwise, May effects are auto-declined.
pub fn resolve_stack_entry_with(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, None)
}

/// Resolve the top entry on the stack with optional decision maker and trigger queue.
///
/// If a trigger_queue is provided, saga lore counters are processed immediately.
/// Otherwise, saga processing must be handled by the caller.
fn resolve_stack_entry_full(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    mut trigger_queue: Option<&mut TriggerQueue>,
) -> Result<(), GameLoopError> {
    let entry = game
        .pop_from_stack()
        .ok_or_else(|| GameLoopError::InvalidState("Stack is empty".to_string()))?;

    // Get the object for this stack entry
    let obj = game.object(entry.object_id).cloned();

    // Create execution context
    // Resolution effects use EventCause::from_effect to distinguish from cost effects
    let mut ctx = ExecutionContext::new(entry.object_id, entry.controller, decision_maker)
        .with_optional_costs_paid(entry.optional_costs_paid.clone())
        .with_cause(EventCause::from_effect(entry.object_id, entry.controller));
    if let Some(x) = entry.x_value {
        ctx = ctx.with_x(x);
    }
    if let Some(defending) = entry.defending_player {
        ctx = ctx.with_defending_player(defending);
    }
    if let Some(triggering_event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(triggering_event);
    }
    if let Some(source_snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(source_snapshot);
    }
    // Pass pre-chosen modes from casting (per MTG rule 601.2b)
    if let Some(ref modes) = entry.chosen_modes {
        ctx = ctx.with_chosen_modes(Some(modes.clone()));
    }
    apply_keyword_payment_tags_for_resolution(game, &entry, &mut ctx);

    // Convert targets and validate them
    // Per MTG Rule 608.2b, if ALL targets are now illegal, the spell/ability fizzles
    let (valid_targets, all_targets_invalid) = validate_stack_entry_targets(game, &entry);

    // If the spell/ability had targets and ALL are now invalid, it fizzles
    if !entry.targets.is_empty() && all_targets_invalid {
        // Spell fizzles - move to graveyard without executing effects
        if let Some(obj) = &obj
            && obj.zone == Zone::Stack
            && !entry.is_ability
        {
            // Move spell to owner's graveyard (via replacement effects)
            let outcome = crate::event_processor::process_zone_change(
                game,
                entry.object_id,
                Zone::Stack,
                Zone::Graveyard,
                &mut *decision_maker,
            );
            if let crate::event_processor::EventOutcome::Proceed(final_zone) = outcome {
                game.move_object(entry.object_id, final_zone);
            }
        }
        return Ok(());
    }

    // Check intervening-if condition at resolution time
    // If the condition is false, the ability does nothing (but doesn't fizzle)
    if let Some(ref condition) = entry.intervening_if
        && let Some(ref triggering_event) = entry.triggering_event
        && !crate::triggers::verify_intervening_if(
            game,
            condition,
            entry.controller,
            triggering_event,
            entry.object_id,
            None,
        )
    {
        // Condition no longer true - ability resolves but does nothing
        return Ok(());
    }
    // If no triggering event is set (shouldn't happen for triggered abilities),
    // we allow the ability to proceed rather than creating a fake event

    ctx = ctx.with_targets(valid_targets);

    // Snapshot target objects for "last known information" before effects execute
    // This allows effects to access power/controller of targets even after they're exiled
    ctx.snapshot_targets(game);

    // Get effects to execute
    // For abilities with stored effects (like triggered abilities), use those directly
    // even if the source object no longer exists (e.g., undying triggers from dead creatures)
    let effects = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = &obj {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        Vec::new()
    };

    // ETB replacement is resolved when the spell actually moves to the battlefield.
    let etb_replacement_result: Option<(bool, bool, Zone)> = None;

    let mut all_events = Vec::new();
    for effect in &effects {
        if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
            all_events.extend(outcome.events);
        }
    }
    // Process events from effect outcomes for triggers
    if let Some(ref mut tq) = trigger_queue {
        for event in all_events {
            let triggers = check_triggers(game, &event);
            for t in triggers {
                tq.add(t);
            }
        }
    }

    // Process pending primitive trigger events emitted by effects and zone changes.
    if let Some(ref mut tq) = trigger_queue {
        drain_pending_trigger_events(game, tq);
    }

    // If this was a saga's final chapter ability, mark it as resolved
    // (the saga will be sacrificed as a state-based action)
    if let Some(saga_id) = entry.saga_final_chapter_source {
        mark_saga_final_chapter_resolved(game, saga_id);
    }

    // Move spell to appropriate zone after resolution
    if let Some(obj) = &obj {
        if obj.zone == Zone::Stack && obj.is_permanent() {
            // Handle ETB replacement: if player didn't satisfy the replacement, redirect
            if let Some((enters, enters_tapped, redirect_zone)) = etb_replacement_result {
                if !enters {
                    // Permanent goes to redirect zone instead of battlefield
                    game.move_object(entry.object_id, redirect_zone);
                    return Ok(());
                }

                // Copy optional_costs_paid to the permanent before moving to battlefield
                if let Some(perm) = game.object_mut(entry.object_id) {
                    perm.optional_costs_paid = entry.optional_costs_paid.clone();
                }

                // Track creature ETBs for trap conditions
                if obj.is_creature() {
                    *game
                        .creatures_entered_this_turn
                        .entry(entry.controller)
                        .or_insert(0) += 1;
                }

                // Interactive replacement was already processed above - skip second ETB processing
                // and move directly to battlefield (avoids double-processing)
                let new_id = game.move_object(entry.object_id, Zone::Battlefield);
                if let Some(id) = new_id {
                    // Apply enters tapped if needed (e.g., shock land not paying life)
                    if enters_tapped {
                        game.tap(id);
                    }

                    if let Some(ref mut tq) = trigger_queue {
                        // Drain pending ZoneChangeEvent emitted by move_object.
                        drain_pending_trigger_events(game, tq);
                    }

                    // Check for ETB triggers
                    if let Some(ref mut tq) = trigger_queue {
                        let etb_event = if enters_tapped {
                            TriggerEvent::new(EnterBattlefieldEvent::tapped(id, Zone::Stack))
                        } else {
                            TriggerEvent::new(EnterBattlefieldEvent::new(id, Zone::Stack))
                        };
                        let etb_triggers = check_triggers(game, &etb_event);
                        for trigger in etb_triggers {
                            tq.add(trigger);
                        }
                    }
                }
                return Ok(());
            }

            // No interactive replacement was handled above - use normal ETB processing
            // Copy optional_costs_paid to the permanent before moving to battlefield
            // (so ETB triggers can access kick count, etc.)
            if let Some(perm) = game.object_mut(entry.object_id) {
                perm.optional_costs_paid = entry.optional_costs_paid.clone();
                // Preserve Convoke/Improvise contributors for later triggered ability resolution.
                perm.keyword_payment_contributions_to_cast =
                    entry.keyword_payment_contributions.clone();
            }

            // Track creature ETBs for trap conditions (before the object moves zones)
            if obj.is_creature() {
                *game
                    .creatures_entered_this_turn
                    .entry(entry.controller)
                    .or_insert(0) += 1;
            }

            // It's a permanent spell, move to battlefield with ETB processing
            // This handles replacement effects like "enters tapped" or "enters with counters"
            let etb_result = game.move_object_with_etb_processing_with_dm(
                entry.object_id,
                Zone::Battlefield,
                decision_maker,
            );

            // Note: Use the new ID from ETB result since zone change creates a new object
            if let Some(result) = etb_result {
                // If this is an Aura, attach it to its target as it enters
                if obj.subtypes.contains(&Subtype::Aura)
                    && let Some(Target::Object(target_id)) = entry
                        .targets
                        .iter()
                        .find(|t| matches!(t, Target::Object(_)))
                {
                    if let Some(aura) = game.object_mut(result.new_id) {
                        aura.attached_to = Some(*target_id);
                    }
                    if let Some(target) = game.object_mut(*target_id)
                        && !target.attachments.contains(&result.new_id)
                    {
                        target.attachments.push(result.new_id);
                    }
                    game.continuous_effects.record_attachment(result.new_id);
                }

                // Check for ETB triggers and add them to the trigger queue
                if let Some(ref mut tq) = trigger_queue {
                    // Drain pending ZoneChangeEvent emitted by ETB move processing.
                    drain_pending_trigger_events(game, tq);

                    let etb_event = if result.enters_tapped {
                        TriggerEvent::new(EnterBattlefieldEvent::tapped(result.new_id, Zone::Stack))
                    } else {
                        TriggerEvent::new(EnterBattlefieldEvent::new(result.new_id, Zone::Stack))
                    };
                    let etb_triggers = check_triggers(game, &etb_event);
                    for trigger in etb_triggers {
                        tq.add(trigger);
                    }
                }

                // If it's a saga, add its initial lore counter and check for chapter triggers
                if obj.subtypes.contains(&Subtype::Saga) {
                    if let Some(tq) = trigger_queue {
                        // Process immediately with the provided trigger queue
                        add_lore_counter_and_check_chapters(game, result.new_id, tq);
                    } else {
                        // Create a temporary trigger queue for immediate processing
                        let mut temp_queue = TriggerQueue::new();
                        add_lore_counter_and_check_chapters(game, result.new_id, &mut temp_queue);
                        // Note: triggers will be put on stack in advance_priority
                    }
                }
            }
        } else if obj.zone == Zone::Stack {
            // It's an instant/sorcery
            // Check if cast with flashback/escape/jump-start/granted escape (exiles after resolution)
            let should_exile = match &entry.casting_method {
                CastingMethod::Normal => false,
                CastingMethod::Alternative(idx) => obj
                    .alternative_casts
                    .get(*idx)
                    .map(|m| m.exiles_after_resolution())
                    .unwrap_or(false),
                CastingMethod::GrantedEscape { .. } => true, // Granted escape always exiles
                CastingMethod::GrantedFlashback => true,     // Granted flashback always exiles
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    ..
                } => {
                    // Check if the alternative cost used exiles after resolution
                    obj.alternative_casts
                        .get(*idx)
                        .map(|m| m.exiles_after_resolution())
                        .unwrap_or(false)
                }
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => {
                    // Normal cost via Yawgmoth's Will - replacement effect handles exile
                    false
                }
            };

            if should_exile {
                game.move_object(entry.object_id, Zone::Exile);
            } else {
                // Process zone change through replacement effects
                // (e.g., Yawgmoth's Will exiles cards going to graveyard)
                let outcome = crate::event_processor::process_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Graveyard,
                    &mut *decision_maker,
                );
                if let crate::event_processor::ZoneChangeOutcome::Proceed(final_zone) = outcome {
                    game.move_object(entry.object_id, final_zone);
                }
            }
        }
        // Abilities just disappear from the stack
    }

    Ok(())
}

/// Get effects for a stack entry.
fn get_effects_for_stack_entry(
    _game: &GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
) -> Vec<Effect> {
    // If this is an ability with stored effects, use those directly
    if let Some(ref effects) = entry.ability_effects {
        return effects.clone();
    }

    // For spells, check the spell_effect field (instants/sorceries)
    if let Some(ref effects) = obj.spell_effect {
        return effects.clone();
    }

    // Permanent spells (creatures, artifacts, enchantments, etc.) don't have effects
    // that execute on resolution - they just enter the battlefield.
    // Don't fall back to looking at their abilities.
    if obj.is_permanent() {
        return Vec::new();
    }

    // For ability stack entries without stored effects, look in the object's abilities
    // (This is a fallback for edge cases, but normally ability_effects should be set)
    for ability in &obj.abilities {
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                return triggered.effects.clone();
            }
            AbilityKind::Activated(activated) => {
                return activated.effects.clone();
            }
            _ => {}
        }
    }

    Vec::new()
}

// ============================================================================
// Saga Support
// ============================================================================

/// Add lore counters to sagas at the start of precombat main phase.
///
/// This should be called once at the start of the precombat main phase, before
/// players receive priority.
///
/// Per MTG rules, this checks the CALCULATED subtypes (after continuous effects)
/// to determine if a permanent is still a Saga. For example, under Blood Moon,
/// Urza's Saga becomes a basic Mountain and loses its Saga subtype, so it
/// won't gain lore counters.
pub fn add_saga_lore_counters(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    let active_player = game.turn.active_player;

    // Collect sagas controlled by active player
    // IMPORTANT: Use calculated_subtypes to check if the permanent is STILL a Saga
    // after continuous effects are applied (e.g., Blood Moon removes Saga subtype)
    let sagas: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Check calculated subtypes (after continuous effects), not base subtypes
            let subtypes = game.calculated_subtypes(id);
            if subtypes.contains(&Subtype::Saga) && obj.controller == active_player {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    for saga_id in sagas {
        add_lore_counter_and_check_chapters(game, saga_id, trigger_queue);
    }
}

/// Add a lore counter to a saga and check for chapter triggers.
///
/// This uses the normal trigger system: adds a lore counter, generates a
/// CounterPlaced event, and lets check_triggers find matching chapter abilities.
/// Chapter triggers use threshold-crossing logic: they fire when the lore count
/// crosses a chapter's threshold, allowing chapters to trigger multiple times
/// if counters are removed and re-added.
pub fn add_lore_counter_and_check_chapters(
    game: &mut GameState,
    saga_id: ObjectId,
    trigger_queue: &mut TriggerQueue,
) {
    // Add lore counter and get the CounterPlaced event
    let Some(event) = game.add_counters(saga_id, CounterType::Lore, 1) else {
        return;
    };

    // Check triggers - this will find any saga chapter abilities that should fire
    // based on whether the threshold was crossed by this counter addition
    let triggers = check_triggers(game, &event);

    // Add triggered abilities to the queue
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
}

/// Mark a saga as having resolved its final chapter.
///
/// Call this after a saga's final chapter ability finishes resolving.
/// The saga will then be sacrificed as a state-based action IF it still has
/// enough lore counters. This function unconditionally marks the saga;
/// the SBA checks the lore counter count before sacrificing.
pub fn mark_saga_final_chapter_resolved(game: &mut GameState, saga_id: ObjectId) {
    if let Some(saga) = game.object(saga_id)
        && saga.subtypes.contains(&Subtype::Saga)
    {
        // Always mark as resolved - the SBA will check lore counters before sacrificing
        game.set_saga_final_chapter_resolved(saga_id);
    }
}

// ============================================================================
// Combat Damage
// ============================================================================

/// Combat damage event for trigger processing.
#[derive(Debug, Clone)]
pub struct CombatDamageEvent {
    /// The source dealing damage.
    pub source: ObjectId,
    /// The target receiving damage.
    pub target: DamageEventTarget,
    /// Amount of damage dealt.
    pub amount: u32,
    /// Amount of life actually lost from this damage (0 for non-player targets, infect, or life-locked players).
    pub life_lost: u32,
    /// The damage result with lifelink/infect info.
    pub result: DamageResult,
}

/// Execute combat damage for a damage step.
///
/// # Arguments
/// * `game` - The game state
/// * `combat` - The combat state
/// * `first_strike` - True for first strike damage step, false for regular
///
/// # Returns
/// A list of damage events that occurred (for trigger processing).
pub fn execute_combat_damage_step(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
) -> Vec<CombatDamageEvent> {
    let mut damage_events = Vec::new();

    // Process each attacker
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;

        // Check if this creature deals damage in this step
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };

        // Use game-aware functions to check abilities from continuous effects
        let participates = if first_strike {
            deals_first_strike_damage_with_game(attacker, game)
        } else {
            deals_regular_combat_damage_with_game(attacker, game)
        };

        if !participates {
            continue;
        }

        // Get attacker's effective power (includes continuous effects).
        let Some(power) = game
            .calculated_power(attacker_id)
            .or_else(|| attacker.power())
        else {
            continue;
        };
        if power <= 0 {
            continue;
        }

        let controller = attacker.controller;

        if is_blocked(combat, attacker_id) {
            // Blocked attacker - deal damage to blockers
            let events =
                deal_damage_to_blockers(game, attacker_id, combat, power as u32, controller);
            damage_events.extend(events);
        } else if is_unblocked(combat, attacker_id) {
            // Unblocked attacker - deal damage to defender
            let event =
                deal_damage_to_defender(game, attacker_id, &attacker_info.target, power as u32);
            if let Some(e) = event {
                damage_events.push(e);
            }
        }
    }

    // Process blockers dealing damage to attackers
    // First, collect all blocker damage info
    let mut blocker_damage_info: Vec<(ObjectId, ObjectId, PlayerId, DamageResult)> = Vec::new();

    for (attacker_id, blocker_ids) in &combat.blockers {
        for &blocker_id in blocker_ids {
            let Some(blocker) = game.object(blocker_id) else {
                continue;
            };

            // Check if this blocker deals damage in this step
            // Use game-aware functions to check abilities from continuous effects
            let participates = if first_strike {
                deals_first_strike_damage_with_game(blocker, game)
            } else {
                deals_regular_combat_damage_with_game(blocker, game)
            };

            if !participates {
                continue;
            }

            // Get blocker's effective power (includes continuous effects).
            let Some(power) = game
                .calculated_power(blocker_id)
                .or_else(|| blocker.power())
            else {
                continue;
            };
            if power <= 0 {
                continue;
            }

            let controller = blocker.controller;

            // Check attacker still exists
            if game.object(*attacker_id).is_none() {
                continue;
            }

            let damage_result = calculate_damage_with_game(
                game,
                blocker,
                DamageTarget::Permanent,
                power as u32,
                true,
            );
            blocker_damage_info.push((blocker_id, *attacker_id, controller, damage_result));
        }
    }

    // Now apply all blocker damage
    for (blocker_id, attacker_id, controller, damage_result) in blocker_damage_info {
        // Apply damage (blocker is dealing damage to attacker)
        let dealt_damage = apply_damage_to_permanent(game, attacker_id, blocker_id, &damage_result);

        // Apply lifelink (through event processing)
        if damage_result.has_lifelink && dealt_damage > 0 {
            let life_to_gain = crate::event_processor::process_life_gain_with_event(
                game,
                controller,
                dealt_damage,
            );
            if life_to_gain > 0
                && let Some(player) = game.player_mut(controller)
            {
                player.gain_life(life_to_gain);
            }
        }

        damage_events.push(CombatDamageEvent {
            source: blocker_id,
            target: DamageEventTarget::Object(attacker_id),
            amount: dealt_damage,
            life_lost: 0,
            result: damage_result,
        });
    }

    damage_events
}

/// Deal damage from an attacker to its blockers.
fn deal_damage_to_blockers(
    game: &mut GameState,
    attacker_id: ObjectId,
    combat: &CombatState,
    total_damage: u32,
    controller: PlayerId,
) -> Vec<CombatDamageEvent> {
    let mut events = Vec::new();

    let blocker_ids = get_damage_assignment_order(combat, attacker_id);
    if blocker_ids.is_empty() {
        return events;
    }

    // Get blocker objects for distribution calculation
    let blockers: Vec<&crate::object::Object> = blocker_ids
        .iter()
        .filter_map(|&id| game.object(id))
        .collect();

    let Some(attacker) = game.object(attacker_id) else {
        return events;
    };

    // Calculate damage distribution (handles trample)
    let (distribution, excess) = distribute_trample_damage(attacker, &blockers, total_damage, game);

    // Get the attack target for potential trample damage
    let attack_target = get_attack_target(combat, attacker_id).cloned();

    // Collect damage results first (while we still have the immutable borrow)
    let mut blocker_damages: Vec<(ObjectId, DamageResult)> = Vec::new();
    for (i, (damage, _is_lethal)) in distribution.iter().enumerate() {
        if *damage == 0 {
            continue;
        }
        let blocker_id = blocker_ids[i];
        let damage_result =
            calculate_damage_with_game(game, attacker, DamageTarget::Permanent, *damage, true);
        blocker_damages.push((blocker_id, damage_result));
    }

    // Calculate excess damage result
    let excess_damage_result = if excess > 0 {
        if let Some(AttackTarget::Player(player_id)) = attack_target {
            Some((
                player_id,
                calculate_damage_with_game(
                    game,
                    attacker,
                    DamageTarget::Player(player_id),
                    excess,
                    true,
                ),
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Now apply all damage (borrow of attacker is dropped)
    for (blocker_id, damage_result) in blocker_damages {
        let dealt_damage = apply_damage_to_permanent(game, blocker_id, attacker_id, &damage_result);

        // Apply lifelink (through event processing)
        if damage_result.has_lifelink && dealt_damage > 0 {
            let life_to_gain = crate::event_processor::process_life_gain_with_event(
                game,
                controller,
                dealt_damage,
            );
            if life_to_gain > 0
                && let Some(player) = game.player_mut(controller)
            {
                player.gain_life(life_to_gain);
            }
        }

        events.push(CombatDamageEvent {
            source: attacker_id,
            target: DamageEventTarget::Object(blocker_id),
            amount: dealt_damage,
            life_lost: 0,
            result: damage_result,
        });
    }

    // Apply excess damage to defending player (trample)
    if let Some((player_id, damage_result)) = excess_damage_result {
        let applied = apply_damage_to_player(game, player_id, attacker_id, &damage_result);

        // Apply lifelink (through event processing)
        if damage_result.has_lifelink && applied.damage_dealt > 0 {
            let life_to_gain = crate::event_processor::process_life_gain_with_event(
                game,
                controller,
                applied.damage_dealt,
            );
            if life_to_gain > 0
                && let Some(player) = game.player_mut(controller)
            {
                player.gain_life(life_to_gain);
            }
        }

        events.push(CombatDamageEvent {
            source: attacker_id,
            target: DamageEventTarget::Player(player_id),
            amount: applied.damage_dealt,
            life_lost: applied.life_lost,
            result: damage_result,
        });
    }

    events
}

/// Deal damage from an unblocked attacker to its target.
fn deal_damage_to_defender(
    game: &mut GameState,
    attacker_id: ObjectId,
    target: &AttackTarget,
    damage: u32,
) -> Option<CombatDamageEvent> {
    let attacker = game.object(attacker_id)?;
    let controller = attacker.controller;

    match target {
        AttackTarget::Player(player_id) => {
            let damage_result = calculate_damage_with_game(
                game,
                attacker,
                DamageTarget::Player(*player_id),
                damage,
                true,
            );

            let applied = apply_damage_to_player(game, *player_id, attacker_id, &damage_result);

            // Apply lifelink (through event processing)
            if damage_result.has_lifelink && applied.damage_dealt > 0 {
                let life_to_gain = crate::event_processor::process_life_gain_with_event(
                    game,
                    controller,
                    applied.damage_dealt,
                );
                if life_to_gain > 0
                    && let Some(player) = game.player_mut(controller)
                {
                    player.gain_life(life_to_gain);
                }
            }

            Some(CombatDamageEvent {
                source: attacker_id,
                target: DamageEventTarget::Player(*player_id),
                amount: applied.damage_dealt,
                life_lost: applied.life_lost,
                result: damage_result,
            })
        }
        AttackTarget::Planeswalker(pw_id) => {
            use crate::event_processor::process_damage_with_event;
            use crate::game_event::DamageTarget as EventDamageTarget;

            let damage_result =
                calculate_damage_with_game(game, attacker, DamageTarget::Permanent, damage, true);

            // Process through replacement/prevention effects
            let (final_damage, was_prevented) = process_damage_with_event(
                game,
                attacker_id,
                EventDamageTarget::Object(*pw_id),
                damage,
                true, // is_combat
            );

            // Damage to planeswalker removes loyalty counters
            if !was_prevented
                && final_damage > 0
                && let Some(pw) = game.object_mut(*pw_id)
            {
                let current_loyalty = pw.counters.get(&CounterType::Loyalty).copied().unwrap_or(0);
                let new_loyalty = current_loyalty.saturating_sub(final_damage);
                if new_loyalty == 0 {
                    pw.counters.remove(&CounterType::Loyalty);
                } else {
                    pw.counters.insert(CounterType::Loyalty, new_loyalty);
                }
            }

            // Apply lifelink (only if damage was dealt, through event processing)
            if !was_prevented && final_damage > 0 && damage_result.has_lifelink {
                let life_to_gain = crate::event_processor::process_life_gain_with_event(
                    game,
                    controller,
                    final_damage,
                );
                if life_to_gain > 0
                    && let Some(player) = game.player_mut(controller)
                {
                    player.gain_life(life_to_gain);
                }
            }

            Some(CombatDamageEvent {
                source: attacker_id,
                target: DamageEventTarget::Object(*pw_id),
                amount: final_damage,
                life_lost: 0,
                result: damage_result,
            })
        }
    }
}

/// Apply damage to a permanent (creature or planeswalker).
///
/// This processes the damage through replacement/prevention effects before applying.
fn apply_damage_to_permanent(
    game: &mut GameState,
    permanent_id: ObjectId,
    source_id: ObjectId,
    result: &DamageResult,
) -> u32 {
    use crate::event_processor::process_damage_with_event;
    use crate::game_event::DamageTarget;

    // Process through replacement/prevention effects
    let (final_damage, was_prevented) = process_damage_with_event(
        game,
        source_id,
        DamageTarget::Object(permanent_id),
        result.damage_dealt,
        true, // is_combat
    );

    if was_prevented || final_damage == 0 {
        return 0;
    }

    if result.has_infect || result.has_wither {
        // Infect/wither: place -1/-1 counters instead of marking damage
        // Scale counters based on final damage vs original damage
        let counter_amount = if result.damage_dealt > 0 {
            (result.minus_counters as u64 * final_damage as u64 / result.damage_dealt as u64) as u32
        } else {
            0
        };
        if let Some(permanent) = game.object_mut(permanent_id) {
            *permanent
                .counters
                .entry(CounterType::MinusOneMinusOne)
                .or_insert(0) += counter_amount;
        }
    } else {
        // Normal damage
        game.mark_damage(permanent_id, final_damage);
    }

    final_damage
}

#[derive(Debug, Clone, Copy)]
struct AppliedPlayerDamage {
    damage_dealt: u32,
    life_lost: u32,
}

/// Apply damage to a player.
///
/// This processes the damage through replacement/prevention effects before applying.
fn apply_damage_to_player(
    game: &mut GameState,
    player_id: PlayerId,
    source_id: ObjectId,
    result: &DamageResult,
) -> AppliedPlayerDamage {
    use crate::event_processor::process_damage_with_event;
    use crate::game_event::DamageTarget;

    // Process through replacement/prevention effects
    let (final_damage, was_prevented) = process_damage_with_event(
        game,
        source_id,
        DamageTarget::Player(player_id),
        result.damage_dealt,
        true, // is_combat
    );

    if was_prevented || final_damage == 0 {
        return AppliedPlayerDamage {
            damage_dealt: 0,
            life_lost: 0,
        };
    }

    let mut life_lost = 0;

    if result.has_infect {
        let Some(player) = game.player_mut(player_id) else {
            return AppliedPlayerDamage {
                damage_dealt: 0,
                life_lost: 0,
            };
        };

        // Infect: give poison counters
        // Scale counters based on final damage vs original damage
        let counter_amount = if result.damage_dealt > 0 {
            (result.poison_counters as u64 * final_damage as u64 / result.damage_dealt as u64)
                as u32
        } else {
            0
        };
        player.poison_counters += counter_amount;
    } else if game.can_change_life_total(player_id)
        && let Some(player) = game.player_mut(player_id)
    {
        // Damage is still dealt even if life total cannot change; life loss tracks only actual life reduction.
        player.life -= final_damage as i32;
        life_lost = final_damage;
    }

    AppliedPlayerDamage {
        damage_dealt: final_damage,
        life_lost,
    }
}

// ============================================================================
// Priority Loop
// ============================================================================

/// Stage of the spell casting process.
///
/// Per MTG Comprehensive Rules 601.2, casting follows this order:
/// 1. Proposing (601.2a) - Move spell to stack
/// 2. ChoosingModes (601.2b) - Announce modes for modal spells
/// 3. ChoosingX (601.2b) - Announce X value
/// 4. ChoosingOptionalCosts (601.2b) - Announce additional costs (kicker, buyback)
/// 5. AnnouncingCost (601.2b) - Announce hybrid/Phyrexian mana choices
/// 6. ChoosingTargets (601.2c) - Choose targets
/// 7. ChoosingExileFromHand - Select cards for alternative costs
/// 8. PayingMana (601.2g-h) - Activate mana abilities and pay costs
/// 9. ReadyToFinalize (601.2i) - Spell becomes cast
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastStage {
    /// Spell is being proposed - moved to stack per 601.2a.
    /// This is the first stage when casting begins.
    Proposing,
    /// Need to choose modes for modal spells (per 601.2b).
    /// Modes must be chosen before targets.
    ChoosingModes,
    /// Need to choose X value (for spells with X in cost).
    ChoosingX,
    /// Need to choose optional costs (kicker, buyback, etc.).
    ChoosingOptionalCosts,
    /// Need to announce hybrid/Phyrexian mana payment choices (per 601.2b).
    /// These choices are locked in before targets are chosen.
    AnnouncingCost,
    /// Need to choose targets.
    ChoosingTargets,
    /// Need to choose cards to exile from hand (for alternative costs like Force of Will).
    ChoosingExileFromHand,
    /// Need to pay mana costs (player can activate mana abilities).
    PayingMana,
    /// Ready to finalize (mana has been paid and spell becomes cast).
    ReadyToFinalize,
}

/// Pending casting method selection for a spell with multiple available methods.
#[derive(Debug, Clone)]
pub struct PendingMethodSelection {
    /// The spell being cast.
    pub spell_id: ObjectId,
    /// The zone the spell is being cast from.
    pub from_zone: Zone,
    /// The player casting the spell.
    pub caster: PlayerId,
    /// The available casting method options.
    pub available_methods: Vec<crate::decision::CastingMethodOption>,
}

/// A spell or ability being cast/activated that needs decisions.
#[derive(Debug, Clone)]
pub struct PendingCast {
    /// The spell/ability being cast.
    pub spell_id: ObjectId,
    /// The zone the spell is being cast from.
    pub from_zone: Zone,
    /// The player casting the spell.
    pub caster: PlayerId,
    /// Current stage of the casting process.
    pub stage: CastStage,
    /// The chosen X value (if applicable).
    pub x_value: Option<u32>,
    /// Targets that have been chosen so far.
    pub chosen_targets: Vec<Target>,
    /// Target requirements that still need to be fulfilled.
    pub remaining_requirements: Vec<TargetRequirement>,
    /// The casting method (normal or alternative like flashback).
    pub casting_method: CastingMethod,
    /// Which optional costs will be paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// Ordered trace of cost payments performed so far.
    pub payment_trace: Vec<CostStep>,
    /// Mana actually spent to cast the spell (color-by-color).
    pub mana_spent_to_cast: ManaPool,
    /// The computed mana cost to pay (set during PayingMana stage).
    pub mana_cost_to_pay: Option<crate::mana::ManaCost>,
    /// Remaining mana pips to pay (pip-by-pip payment flow).
    /// Each element is a pip with its alternatives (e.g., [Black, Life(2)] for {B/P}).
    pub remaining_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Cards chosen to exile from hand as part of alternative costs.
    pub cards_to_exile: Vec<ObjectId>,
    /// Pre-chosen modes for modal spells (per MTG rule 601.2b).
    /// Set during ChoosingModes stage, used during resolution.
    pub chosen_modes: Option<Vec<usize>>,
    /// Hybrid/Phyrexian mana payment choices made during cost announcement (601.2b).
    /// Maps pip index to the chosen mana symbol for that pip.
    pub hybrid_choices: Vec<(usize, crate::mana::ManaSymbol)>,
    /// Hybrid/Phyrexian pips that still need announcement (601.2b).
    /// Each element is (pip_index, alternatives). Processed one at a time.
    pub pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    /// The spell's ObjectId on the stack (after being moved per 601.2a).
    pub stack_id: ObjectId,
    /// Permanents that contributed keyword-ability alternative payments while casting this spell.
    pub keyword_payment_contributions: Vec<KeywordPaymentContribution>,
}

/// Stage of the ability activation process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationStage {
    /// Need to choose X value for abilities with X in cost.
    ChoosingX,
    /// Need to choose sacrifice targets.
    ChoosingSacrifice,
    /// Need to announce hybrid/Phyrexian mana payment choices (per MTG rule 601.2b via 602.2b).
    AnnouncingCost,
    /// Need to choose ability targets.
    ChoosingTargets,
    /// Need to pay mana costs (player can activate mana abilities).
    PayingMana,
    /// Ready to finalize (costs paid, ability goes on stack).
    ReadyToFinalize,
}

/// An activated ability being activated that needs decisions.
#[derive(Debug, Clone)]
pub struct PendingActivation {
    /// The source permanent of the activated ability.
    pub source: ObjectId,
    /// Index of the ability being activated.
    pub ability_index: usize,
    /// The player activating the ability.
    pub activator: PlayerId,
    /// Current stage of the activation process.
    pub stage: ActivationStage,
    /// The effects of the ability.
    pub effects: Vec<crate::effect::Effect>,
    /// Targets that have been chosen so far.
    pub chosen_targets: Vec<Target>,
    /// Target requirements that still need to be fulfilled.
    pub remaining_requirements: Vec<TargetRequirement>,
    /// The computed mana cost to pay.
    pub mana_cost_to_pay: Option<crate::mana::ManaCost>,
    /// Ordered trace of cost payments performed so far.
    pub payment_trace: Vec<CostStep>,
    /// Remaining mana pips to pay (pip-by-pip payment flow).
    /// Each element is a pip with its alternatives (e.g., [Black, Life(2)] for {B/P}).
    pub remaining_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Remaining sacrifice costs to pay: (filter, description).
    pub remaining_sacrifice_costs: Vec<(crate::cost::PermanentFilter, String)>,
    /// Whether this ability is once per turn (needs recording).
    pub is_once_per_turn: bool,
    /// Stable instance ID of the source (persists across zone changes).
    pub source_stable_id: StableId,
    /// Last known information for the source at activation time.
    pub source_snapshot: ObjectSnapshot,
    /// Name of the source for display purposes.
    pub source_name: String,
    /// The chosen X value for abilities with X in cost.
    pub x_value: Option<usize>,
    /// Hybrid/Phyrexian mana choices made during AnnouncingCost stage (per MTG rule 601.2b via 602.2b).
    /// Each element is (pip_index, chosen_symbol).
    pub hybrid_choices: Vec<(usize, crate::mana::ManaSymbol)>,
    /// Pending hybrid/Phyrexian pips that still need announcement.
    /// Each element is (pip_index, alternatives).
    pub pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
}

/// A mana ability being activated that needs mana payment first.
///
/// Mana abilities don't use the stack, but if they have a mana cost
/// (like Blood Celebrant's {B}), we need to let the player tap mana sources first.
#[derive(Debug, Clone)]
pub struct PendingManaAbility {
    /// The source permanent of the mana ability.
    pub source: ObjectId,
    /// Index of the ability being activated.
    pub ability_index: usize,
    /// The player activating the ability.
    pub activator: PlayerId,
    /// The mana cost that needs to be paid.
    pub mana_cost: crate::mana::ManaCost,
    /// Other (non-mana) costs that have already been validated.
    pub other_costs: Vec<crate::costs::Cost>,
    /// The mana symbols to add (for simple mana abilities).
    pub mana_to_add: Vec<crate::mana::ManaSymbol>,
    /// The effects to execute (for complex mana abilities like Blood Celebrant).
    pub effects: Option<Vec<crate::effect::Effect>>,
}

/// State for tracking the priority loop between decisions.
#[derive(Debug, Clone)]
pub struct PriorityLoopState {
    tracker: PriorityTracker,
    /// A pending spell cast waiting for target selection.
    pub pending_cast: Option<PendingCast>,
    /// A pending ability activation waiting for cost payment.
    pub pending_activation: Option<PendingActivation>,
    /// A pending casting method selection for spells with multiple available methods.
    pub pending_method_selection: Option<PendingMethodSelection>,
    /// A pending mana ability activation waiting for mana payment.
    pub pending_mana_ability: Option<PendingManaAbility>,
    /// Checkpoint of game state saved when starting an action chain.
    /// If an error occurs during the chain, we restore to this state.
    pub checkpoint: Option<GameState>,
    /// Whether pip-by-pip mana payment should auto-pick a single legal option.
    /// CLI/tests can keep this enabled for speed; WASM UI can disable it to require explicit taps.
    pub auto_choose_single_pip_payment: bool,
}

impl PriorityLoopState {
    /// Create a new priority loop state.
    pub fn new(num_players: usize) -> Self {
        Self {
            tracker: PriorityTracker::new(num_players),
            pending_cast: None,
            pending_activation: None,
            pending_method_selection: None,
            pending_mana_ability: None,
            checkpoint: None,
            auto_choose_single_pip_payment: true,
        }
    }

    /// Save a checkpoint of the current game state.
    /// This should be called when starting an action chain (cast spell, activate ability).
    pub fn save_checkpoint(&mut self, game: &GameState) {
        self.checkpoint = Some(game.clone());
    }

    /// Clear the checkpoint (called when action completes successfully or after restore).
    pub fn clear_checkpoint(&mut self) {
        self.checkpoint = None;
    }

    /// Check if there's an active action chain (pending cast or activation).
    pub fn has_pending_action(&self) -> bool {
        self.pending_cast.is_some()
            || self.pending_activation.is_some()
            || self.pending_method_selection.is_some()
    }

    /// Configure whether single-option pip payments should be auto-selected.
    pub fn set_auto_choose_single_pip_payment(&mut self, enabled: bool) {
        self.auto_choose_single_pip_payment = enabled;
    }

    /// Reset pass tracking and assign priority to the active player for a fresh priority window.
    pub fn reset_for_new_priority_window(&mut self, game: &mut GameState) {
        self.tracker.set_players_in_game(game.players_in_game());
        reset_priority(game, &mut self.tracker);
    }
}

/// Advance the priority loop until a decision is needed or phase ends.
///
/// This is the main entry point for the decision-based game loop.
/// Call this repeatedly, handling decisions as they come, until
/// it returns `GameProgress::Continue` (phase ends) or `GameProgress::GameOver`.
pub fn advance_priority(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<GameProgress, GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    advance_priority_with_dm(game, trigger_queue, &mut dm)
}

/// Advance priority with a decision maker for triggered ability targeting.
///
/// This version allows proper target selection for triggered abilities.
fn advance_priority_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending replacement effect choice first
    // This takes priority over normal game flow
    if let Some(pending) = &game.pending_replacement_choice {
        let options: Vec<ReplacementOption> = pending
            .applicable_effects
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                game.replacement_effects
                    .get_effect(*id)
                    .map(|e| ReplacementOption {
                        index: i,
                        source: e.source,
                        description: format!("{:?}", e.replacement),
                    })
            })
            .collect();

        // Convert to SelectOptionsContext for replacement effect choice
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
            })
            .collect();
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            pending.player,
            None,
            "Choose replacement effect to apply",
            selectable_options,
            1,
            1,
        );
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ));
    }

    // Check and apply state-based actions
    check_and_apply_sbas(game, trigger_queue)?;

    // Put triggered abilities on the stack with target selection
    put_triggers_on_stack_with_dm(game, trigger_queue, decision_maker)?;

    // Check if game is over
    let remaining: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.is_in_game())
        .map(|p| p.id)
        .collect();

    if remaining.is_empty() {
        return Ok(GameProgress::GameOver(GameResult::Draw));
    }
    if remaining.len() == 1 {
        return Ok(GameProgress::GameOver(GameResult::Winner(remaining[0])));
    }

    // Get current priority player
    let Some(priority_player) = game.turn.priority_player else {
        // No one has priority, phase should end
        return Ok(GameProgress::Continue);
    };

    // Compute legal actions for the priority player
    let legal_actions = compute_legal_actions(game, priority_player);
    let commander_actions = compute_commander_actions(game, priority_player);

    // Return decision for the player using the new context-based system
    let ctx = crate::decisions::context::PriorityContext::new(
        priority_player,
        legal_actions,
        commander_actions,
    );
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Priority(ctx),
    ))
}

/// Apply a player's response to a decision during the priority loop.
///
/// This handles both `PriorityAction` responses (for normal priority decisions)
/// and `Targets` responses (when a spell is being cast and needs targets).
pub fn apply_priority_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
) -> Result<GameProgress, GameLoopError> {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    apply_priority_response_with_dm(game, trigger_queue, state, response, &mut auto_dm)
}

/// Apply a player's response to a decision during the priority loop, with an optional decision maker.
///
/// The decision maker is used for ETB replacement effects that require player input
/// (like Mox Diamond asking whether to discard a land).
pub fn apply_priority_response_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let PriorityResponse::Attackers(declarations) = response {
        if game.turn.step != Some(Step::DeclareAttackers) {
            return Err(GameLoopError::InvalidState(
                "Attackers response outside Declare Attackers step".to_string(),
            ));
        }
        let mut combat = game.combat.take().unwrap_or_default();
        let result = apply_attacker_declarations(game, &mut combat, trigger_queue, declarations);
        game.combat = Some(combat);
        result?;
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    if let PriorityResponse::Blockers {
        defending_player,
        declarations,
    } = response
    {
        if game.turn.step != Some(Step::DeclareBlockers) {
            return Err(GameLoopError::InvalidState(
                "Blockers response outside Declare Blockers step".to_string(),
            ));
        }
        let mut combat = game.combat.take().ok_or_else(|| {
            GameLoopError::InvalidState("Combat state missing at declare blockers".to_string())
        })?;
        let result = apply_blocker_declarations(
            game,
            &mut combat,
            trigger_queue,
            declarations,
            *defending_player,
        );
        game.combat = Some(combat);
        result?;
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    // Handle replacement effect choice
    if let PriorityResponse::ReplacementChoice(index) = response {
        return apply_replacement_choice_response(game, trigger_queue, *index, decision_maker);
    }

    // Handle target selection for a pending cast
    if let PriorityResponse::Targets(targets) = response {
        return apply_targets_response(game, trigger_queue, state, targets, &mut *decision_maker);
    }

    // Handle X value selection for a pending cast
    if let PriorityResponse::XValue(x) | PriorityResponse::NumberChoice(x) = response {
        return apply_x_value_response(game, trigger_queue, state, *x, &mut *decision_maker);
    }

    // Handle mode selection for a pending cast (per MTG rule 601.2b, modes before optional costs)
    if let PriorityResponse::Modes(modes) = response
        && state.pending_cast.is_some()
    {
        return apply_modes_response(game, trigger_queue, state, modes, &mut *decision_maker);
    }

    // Handle optional costs selection for a pending cast
    if let PriorityResponse::OptionalCosts(choices) = response {
        return apply_optional_costs_response(
            game,
            trigger_queue,
            state,
            choices,
            &mut *decision_maker,
        );
    }

    // Handle mana payment selection for a pending cast, activation, or mana ability
    if let PriorityResponse::ManaPayment(choice) = response {
        // Check for pending mana ability first (most specific)
        if state.pending_mana_ability.is_some() {
            return apply_mana_payment_response_mana_ability(
                game,
                trigger_queue,
                state,
                *choice,
                decision_maker,
            );
        }
        // Check for pending activation
        if state.pending_activation.is_some() {
            return apply_mana_payment_response_activation(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        return apply_mana_payment_response(
            game,
            trigger_queue,
            state,
            *choice,
            &mut *decision_maker,
        );
    }

    // Handle pip-by-pip mana payment for a pending activation or cast
    if let PriorityResponse::ManaPipPayment(choice) = response {
        if state.pending_activation.is_some() {
            return apply_pip_payment_response_activation(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        if state.pending_cast.is_some() {
            return apply_pip_payment_response_cast(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        return Err(GameLoopError::InvalidState(
            "ManaPipPayment response but no pending activation or cast".to_string(),
        ));
    }

    // Handle sacrifice target selection for a pending activation
    if let PriorityResponse::SacrificeTarget(target_id) = response {
        return apply_sacrifice_target_response(
            game,
            trigger_queue,
            state,
            *target_id,
            &mut *decision_maker,
        );
    }

    // Handle card to exile selection for a pending cast with alternative cost
    if let PriorityResponse::CardToExile(card_id) = response {
        return apply_card_to_exile_response(
            game,
            trigger_queue,
            state,
            *card_id,
            &mut *decision_maker,
        );
    }

    // Handle hybrid/Phyrexian mana choice for a pending cast (per MTG rule 601.2b)
    if let PriorityResponse::HybridChoice(choice) = response {
        return apply_hybrid_choice_response(
            game,
            trigger_queue,
            state,
            *choice,
            &mut *decision_maker,
        );
    }

    // Handle casting method selection for a pending spell with multiple methods
    if let PriorityResponse::CastingMethodChoice(choice_idx) = response {
        return apply_casting_method_choice_response(
            game,
            trigger_queue,
            state,
            *choice_idx,
            &mut *decision_maker,
        );
    }

    let PriorityResponse::PriorityAction(action) = response else {
        return Err(ResponseError::WrongResponseType.into());
    };

    match action {
        LegalAction::PassPriority => {
            let result = pass_priority(game, &mut state.tracker);

            match result {
                PriorityResult::Continue => {
                    // Next player gets priority, advance again
                    // Use decision maker for triggered ability targeting if available
                    advance_priority_with_dm(game, trigger_queue, decision_maker)
                }
                PriorityResult::StackResolves => {
                    // Resolve top of stack, passing decision maker for ETB replacements, choices, etc.
                    resolve_stack_entry_with_dm_and_triggers(game, decision_maker, trigger_queue)?;
                    // Reset priority to active player
                    reset_priority(game, &mut state.tracker);
                    // Signal that stack resolved - outer loop will call advance_priority_with_dm
                    // with the proper decision maker for trigger target selection
                    Ok(GameProgress::StackResolved)
                }
                PriorityResult::PhaseEnds => Ok(GameProgress::Continue),
            }
        }
        LegalAction::PlayLand { land_id } => {
            // Play the land with ETB replacement handling
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            let action = crate::special_actions::SpecialAction::PlayLand { card_id: *land_id };

            // Validate that the player can play the land
            crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                .map_err(|e| GameLoopError::InvalidState(format!("Cannot play land: {:?}", e)))?;

            let old_zone = game.object(*land_id).map(|o| o.zone).unwrap_or(Zone::Hand);
            let result = game
                .move_object_with_etb_processing_with_dm(
                    *land_id,
                    Zone::Battlefield,
                    decision_maker,
                )
                .ok_or_else(|| GameLoopError::InvalidState("Failed to move land".to_string()))?;
            let new_id = result.new_id;

            // Set controller
            if let Some(obj) = game.object_mut(new_id) {
                obj.controller = player;
            }

            // Check for ETB triggers only if the land entered the battlefield.
            if game
                .object(new_id)
                .map(|o| o.zone == Zone::Battlefield)
                .unwrap_or(false)
            {
                // Drain pending ZoneChangeEvent emitted by ETB move processing.
                drain_pending_trigger_events(game, trigger_queue);

                let etb_event = if result.enters_tapped {
                    TriggerEvent::new(EnterBattlefieldEvent::tapped(new_id, old_zone))
                } else {
                    TriggerEvent::new(EnterBattlefieldEvent::new(new_id, old_zone))
                };
                let etb_triggers = check_triggers(game, &etb_event);
                for trigger in etb_triggers {
                    trigger_queue.add(trigger);
                }
            }

            // Mark that the player has played a land this turn
            if let Some(player_data) = game.player_mut(player) {
                player_data.record_land_play();
            }

            // Player retains priority after playing a land
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => {
            // Save checkpoint before starting the action chain
            // This allows rollback if the player makes an invalid choice
            state.save_checkpoint(game);

            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            // Check if there are multiple available casting methods for this spell
            // and prompt for selection if the action uses the Normal method (i.e., user selected the spell generally)
            if matches!(casting_method, CastingMethod::Normal) {
                let available_methods =
                    collect_available_casting_methods(game, player, *spell_id, *from_zone);
                if available_methods.len() > 1 {
                    // Store the pending selection and prompt user
                    state.pending_method_selection = Some(PendingMethodSelection {
                        spell_id: *spell_id,
                        from_zone: *from_zone,
                        caster: player,
                        available_methods: available_methods.clone(),
                    });

                    // Convert to SelectOptionsContext for casting method choice
                    let selectable_options: Vec<crate::decisions::context::SelectableOption> =
                        available_methods
                            .iter()
                            .enumerate()
                            .map(|(i, opt)| {
                                crate::decisions::context::SelectableOption::new(
                                    i,
                                    format!("{}: {}", opt.name, opt.cost_description),
                                )
                            })
                            .collect();
                    let spell_name = game
                        .object(*spell_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| "spell".to_string());
                    let ctx = crate::decisions::context::SelectOptionsContext::new(
                        player,
                        Some(*spell_id),
                        format!("Choose casting method for {}", spell_name),
                        selectable_options,
                        1,
                        1,
                    );
                    return Ok(GameProgress::NeedsDecisionCtx(
                        crate::decisions::context::DecisionContext::SelectOptions(ctx),
                    ));
                }
            }

            // Move spell to stack immediately per MTG rule 601.2a
            // This happens at the start of proposal, before any choices are made
            let stack_id = propose_spell_cast(game, *spell_id, *from_zone, player)?;

            // Get the spell's mana cost and effects, considering casting method
            // Note: We use stack_id now since the spell has been moved to stack
            let (mana_cost, effects) = if let Some(obj) = game.object(stack_id) {
                let cost = match casting_method {
                    CastingMethod::Normal => obj.mana_cost.clone(),
                    CastingMethod::Alternative(idx) => {
                        if let Some(method) = obj.alternative_casts.get(*idx) {
                            // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                            // For other methods (flashback, etc.), fall back to spell's cost.
                            if !method.cost_effects().is_empty() {
                                method.mana_cost().cloned()
                            } else {
                                method
                                    .mana_cost()
                                    .cloned()
                                    .or_else(|| obj.mana_cost.clone())
                            }
                        } else {
                            obj.mana_cost.clone()
                        }
                    }
                    CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(), // Use card's own cost
                    CastingMethod::GrantedFlashback => obj.mana_cost.clone(), // Use card's own cost
                    CastingMethod::PlayFrom {
                        use_alternative: None,
                        ..
                    } => {
                        // Yawgmoth's Will normal cost - use card's mana cost
                        obj.mana_cost.clone()
                    }
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        ..
                    } => {
                        // Yawgmoth's Will with alternative cost (like Force of Will's pitch)
                        if let Some(method) = obj.alternative_casts.get(*idx) {
                            // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                            if !method.cost_effects().is_empty() {
                                method.mana_cost().cloned()
                            } else {
                                method
                                    .mana_cost()
                                    .cloned()
                                    .or_else(|| obj.mana_cost.clone())
                            }
                        } else {
                            obj.mana_cost.clone()
                        }
                    }
                };
                (cost, obj.spell_effect.clone().unwrap_or_default())
            } else {
                (None, Vec::new())
            };

            // Check if spell has X in cost
            let has_x = mana_cost.as_ref().map(|cost| cost.has_x()).unwrap_or(false);

            if has_x {
                // Need to choose X value first - use potential mana (pool + untapped sources)
                let max_x = if let Some(ref cost) = mana_cost {
                    let allow_any_color = game.can_spend_mana_as_any_color(player, Some(stack_id));
                    compute_potential_mana(game, player)
                        .max_x_for_cost_with_any_color(cost, allow_any_color)
                } else {
                    0
                };

                // Extract target requirements for later (use stack_id since spell is on stack)
                let requirements =
                    extract_target_requirements(game, &effects, player, Some(stack_id));

                // Initialize optional costs tracker from the spell's optional costs
                let optional_costs_paid = game
                    .object(stack_id)
                    .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
                    .unwrap_or_default();

                state.pending_cast = Some(PendingCast {
                    spell_id: stack_id, // Use stack_id since spell is now on stack
                    from_zone: *from_zone,
                    caster: player,
                    stage: CastStage::ChoosingX,
                    x_value: None,
                    chosen_targets: Vec::new(),
                    remaining_requirements: requirements,
                    casting_method: casting_method.clone(),
                    optional_costs_paid,
                    payment_trace: Vec::new(),
                    mana_spent_to_cast: ManaPool::default(),
                    mana_cost_to_pay: None,
                    remaining_mana_pips: Vec::new(),
                    cards_to_exile: Vec::new(),
                    chosen_modes: None,
                    hybrid_choices: Vec::new(),
                    pending_hybrid_pips: Vec::new(),
                    stack_id,
                    keyword_payment_contributions: Vec::new(),
                });

                let ctx = crate::decisions::context::NumberContext::x_value(
                    player, stack_id, // Use stack_id
                    max_x,
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Number(ctx),
                ))
            } else {
                // No X cost, check for optional costs then targets
                let requirements =
                    extract_target_requirements(game, &effects, player, Some(stack_id));

                // Initialize optional costs tracker from the spell's optional costs
                let optional_costs_paid = game
                    .object(stack_id)
                    .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
                    .unwrap_or_default();

                let pending = PendingCast {
                    spell_id: stack_id, // Use stack_id since spell is now on stack
                    from_zone: *from_zone,
                    caster: player,
                    stage: CastStage::ChoosingModes, // Will be updated by helper
                    x_value: None,
                    chosen_targets: Vec::new(),
                    remaining_requirements: requirements,
                    casting_method: casting_method.clone(),
                    optional_costs_paid,
                    payment_trace: Vec::new(),
                    mana_spent_to_cast: ManaPool::default(),
                    mana_cost_to_pay: None,
                    remaining_mana_pips: Vec::new(),
                    cards_to_exile: Vec::new(),
                    chosen_modes: None,
                    hybrid_choices: Vec::new(),
                    pending_hybrid_pips: Vec::new(),
                    stack_id,
                    keyword_payment_contributions: Vec::new(),
                };

                check_modes_or_continue(game, trigger_queue, state, pending, &mut *decision_maker)
            }
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            // Re-check activation legality at execution time so stale actions can’t
            // bypass constraints discovered after action discovery.
            if let Some(obj) = game.object(*source) {
                if let Some(ability) = obj.abilities.get(*ability_index) {
                    if let AbilityKind::Activated(activated) = &ability.kind {
                        if !can_activate_ability_with_restrictions(
                            game,
                            *source,
                            *ability_index,
                            activated,
                        ) {
                            return Err(GameLoopError::InvalidState(
                                "Ability activation restrictions are no longer satisfied"
                                    .to_string(),
                            ));
                        }
                    } else {
                        return Err(GameLoopError::InvalidState(
                            "Selected action is not an activated ability".to_string(),
                        ));
                    }
                } else {
                    return Err(GameLoopError::InvalidState(
                        "Ability index no longer valid".to_string(),
                    ));
                }
            } else {
                return Err(GameLoopError::InvalidState(
                    "Ability source no longer exists".to_string(),
                ));
            }

            // Save checkpoint before starting the action chain
            // This allows rollback if the player makes an invalid choice
            state.save_checkpoint(game);

            // Get the ability cost, effects, tracking info, and source info for the stack entry
            let (cost, effects, is_turn_capped, source_stable_id, source_name, source_snapshot) =
                if let Some(obj) = game.object(*source) {
                    let stable_id = obj.stable_id;
                    let name = obj.name.clone();
                    let snapshot =
                        ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
                    if let Some(ability) = obj.abilities.get(*ability_index) {
                        if let AbilityKind::Activated(activated) = &ability.kind {
                            let is_turn_capped = activated.max_activations_per_turn().is_some();
                            (
                                activated.mana_cost.clone(),
                                activated.effects.clone(),
                                is_turn_capped,
                                stable_id,
                                name,
                                snapshot,
                            )
                        } else {
                            (
                                crate::cost::TotalCost::free(),
                                Vec::new(),
                                false,
                                stable_id,
                                name,
                                snapshot,
                            )
                        }
                    } else {
                        (
                            crate::cost::TotalCost::free(),
                            Vec::new(),
                            false,
                            stable_id,
                            name,
                            snapshot,
                        )
                    }
                } else {
                    // Source doesn't exist - return error or use defaults
                    return Err(GameLoopError::InvalidState(
                        "Ability source no longer exists".to_string(),
                    ));
                };

            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            // Pay immediate costs and collect costs that need choices
            let mut mana_cost_to_pay: Option<crate::mana::ManaCost> = None;
            let mut sacrifice_costs: Vec<(PermanentFilter, String)> = Vec::new();
            let mut payment_trace: Vec<CostStep> = Vec::new();

            let mut cost_ctx = CostContext::new(*source, player, &mut *decision_maker);

            for cost_component in cost.costs().iter() {
                use crate::costs::CostProcessingMode;

                match cost_component.processing_mode() {
                    CostProcessingMode::InlineWithTriggers => {
                        // Sacrifice self - handle inline for trigger detection
                        if game.object(*source).is_some() {
                            let snapshot = game
                                .object(*source)
                                .map(|obj| ObjectSnapshot::from_object(obj, game));
                            let sacrificing_player = snapshot
                                .as_ref()
                                .map(|snap| snap.controller)
                                .or(Some(player));
                            game.move_object(*source, Zone::Graveyard);
                            game.queue_trigger_event(TriggerEvent::new(
                                SacrificeEvent::new(*source, Some(*source))
                                    .with_snapshot(snapshot, sacrificing_player),
                            ));
                            drain_pending_trigger_events(game, trigger_queue);

                            #[cfg(feature = "net")]
                            {
                                // Record sacrifice payment for deterministic trace
                                payment_trace.push(CostStep::Payment(CostPayment::Sacrifice {
                                    objects: vec![GameObjectId(source.0)],
                                }));
                            }
                        }
                    }
                    CostProcessingMode::ManaPayment { cost } => {
                        // Save mana cost for later payment through mana payment UI
                        mana_cost_to_pay = Some(cost);
                    }
                    CostProcessingMode::SacrificeTarget { filter } => {
                        // Collect sacrifice costs that need target selection
                        let desc = cost_component.processing_mode().display();
                        sacrifice_costs.push((filter, desc));
                    }
                    CostProcessingMode::Immediate => {
                        // Immediate costs (tap, untap, life, remove counters, etc.)
                        if cost_component.pay(game, &mut cost_ctx).is_err() {
                            // Cost payment failed - shouldn't happen if can_pay was checked
                        } else {
                            record_immediate_cost_payment(
                                &mut payment_trace,
                                cost_component,
                                *source,
                            );
                        }
                    }
                    CostProcessingMode::DiscardCards { .. }
                    | CostProcessingMode::ExileFromHand { .. } => {
                        // Legacy no-op: activation costs using discard/exile-from-hand are
                        // represented as cost_effects and handled in the cost-effect path.
                    }
                }
            }
            drain_pending_trigger_events(game, trigger_queue);

            // Extract target requirements from the ability effects
            let target_requirements =
                extract_target_requirements(game, &effects, player, Some(*source));

            // Check if mana cost has X
            let has_x = mana_cost_to_pay
                .as_ref()
                .map(|c| c.has_x())
                .unwrap_or(false);

            // Check for hybrid/Phyrexian pips requiring announcement (per MTG rule 601.2b via 602.2b)
            let pips_to_announce = mana_cost_to_pay
                .as_ref()
                .map(get_pips_requiring_announcement)
                .unwrap_or_default();
            let has_hybrid_pips = !pips_to_announce.is_empty();

            // Create pending activation if there are choices to make
            if has_x
                || !sacrifice_costs.is_empty()
                || has_hybrid_pips
                || !target_requirements.is_empty()
                || mana_cost_to_pay.is_some()
            {
                // Determine starting stage (per MTG rule 602.2b, follows 601.2b-h order)
                // Order: X value → Sacrifice → Hybrid/Phyrexian announcement → Targets → Mana payment
                let stage = if has_x {
                    ActivationStage::ChoosingX
                } else if !sacrifice_costs.is_empty() {
                    ActivationStage::ChoosingSacrifice
                } else if has_hybrid_pips {
                    ActivationStage::AnnouncingCost
                } else if !target_requirements.is_empty() {
                    ActivationStage::ChoosingTargets
                } else {
                    ActivationStage::PayingMana
                };

                let pending = PendingActivation {
                    source: *source,
                    ability_index: *ability_index,
                    activator: player,
                    stage,
                    effects: effects.to_vec(),
                    chosen_targets: Vec::new(),
                    remaining_requirements: target_requirements,
                    mana_cost_to_pay,
                    payment_trace,
                    remaining_mana_pips: Vec::new(), // Populated when entering PayingMana stage
                    remaining_sacrifice_costs: sacrifice_costs,
                    is_once_per_turn: is_turn_capped,
                    source_stable_id,
                    source_name,
                    source_snapshot,
                    x_value: None,
                    hybrid_choices: Vec::new(),
                    pending_hybrid_pips: pips_to_announce,
                };

                continue_activation(game, trigger_queue, state, pending, &mut *decision_maker)
            } else {
                // No choices needed - put ability on stack directly
                if is_turn_capped {
                    game.record_ability_activation(*source, *ability_index);
                }

                let entry = StackEntry::ability(*source, player, effects.to_vec())
                    .with_source_info(source_stable_id, source_name)
                    .with_source_snapshot(source_snapshot);
                game.push_to_stack(entry);
                queue_ability_activated_event(
                    game,
                    trigger_queue,
                    *source,
                    player,
                    false,
                    Some(source_stable_id),
                );

                reset_priority(game, &mut state.tracker);
                advance_priority_with_dm(game, trigger_queue, decision_maker)
            }
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            // Mana abilities don't use the stack
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            if let Some(obj) = game.object(*source)
                && let Some(ability) = obj.abilities.get(*ability_index)
                && let AbilityKind::Mana(mana_ability) = &ability.kind
            {
                let mana_to_add = mana_ability.mana.clone();
                let effects_to_run = mana_ability.effects.clone();
                let cost = mana_ability.mana_cost.clone();

                // Separate mana costs from other costs
                let mut mana_cost: Option<crate::mana::ManaCost> = None;
                let mut other_costs: Vec<crate::costs::Cost> = Vec::new();

                for c in cost.costs() {
                    if let Some(mc) = c.processing_mode().mana_cost() {
                        mana_cost = Some(mc.clone());
                    } else {
                        other_costs.push(c.clone());
                    }
                }

                // Check if we can pay the mana cost from current pool
                let can_pay_mana = if let Some(ref mc) = mana_cost {
                    game.can_pay_mana_cost(player, Some(*source), mc, 0)
                } else {
                    true // No mana cost
                };

                if can_pay_mana {
                    // Pay all costs immediately
                    let mut cost_ctx = CostContext::new(*source, player, &mut *decision_maker);

                    // Pay mana cost first
                    if let Some(ref mc) = mana_cost
                        && !game.try_pay_mana_cost(player, Some(*source), mc, 0)
                    {
                        return Err(GameLoopError::InvalidState(
                            "Failed to pay mana cost".to_string(),
                        ));
                    }

                    // Pay other costs (from TotalCost, not cost_effects)
                    for c in &other_costs {
                        crate::special_actions::pay_cost_component_with_choice(
                            game,
                            c,
                            &mut cost_ctx,
                        )
                        .map_err(|e| {
                            GameLoopError::InvalidState(format!("Failed to pay cost: {:?}", e))
                        })?;
                    }
                    drain_pending_trigger_events(game, trigger_queue);

                    // Execute the mana ability effects
                    if let Some(effects) = effects_to_run {
                        let mut ctx = ExecutionContext::new(*source, player, &mut *decision_maker);
                        let mut emitted_events = Vec::new();

                        for effect in &effects {
                            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                                emitted_events.extend(outcome.events);
                            }
                        }
                        queue_triggers_for_events(game, trigger_queue, emitted_events);
                        drain_pending_trigger_events(game, trigger_queue);
                    } else {
                        // Add fixed mana to player's pool
                        if let Some(player_obj) = game.player_mut(player) {
                            for symbol in &mana_to_add {
                                player_obj.mana_pool.add(*symbol, 1);
                            }
                        }
                    }

                    queue_ability_activated_event(game, trigger_queue, *source, player, true, None);

                    // Player retains priority after activating mana ability
                    return advance_priority_with_dm(game, trigger_queue, decision_maker);
                } else {
                    // Need to tap lands / activate mana abilities to pay the mana cost
                    // Create a pending mana ability and show PayMana decision
                    let source_name = game
                        .object(*source)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let context = format!("{}'s ability", source_name);

                    let pending = PendingManaAbility {
                        source: *source,
                        ability_index: *ability_index,
                        activator: player,
                        mana_cost: mana_cost.unwrap_or_default(),
                        other_costs,
                        mana_to_add,
                        effects: effects_to_run,
                    };

                    let options = compute_mana_ability_payment_options(
                        game,
                        player,
                        &pending,
                        &mut *decision_maker,
                    );
                    state.pending_mana_ability = Some(pending);

                    // Convert ManaPaymentOption to SelectableOption
                    let selectable_options: Vec<crate::decisions::context::SelectableOption> =
                        options
                            .iter()
                            .map(|opt| {
                                crate::decisions::context::SelectableOption::new(
                                    opt.index,
                                    &opt.description,
                                )
                            })
                            .collect();

                    let ctx = crate::decisions::context::SelectOptionsContext::mana_payment(
                        player,
                        *source,
                        context,
                        selectable_options,
                    );
                    return Ok(GameProgress::NeedsDecisionCtx(
                        crate::decisions::context::DecisionContext::SelectOptions(ctx),
                    ));
                }
            }

            // Player retains priority after activating mana ability
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::TurnFaceUp { creature_id } => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            let action = crate::special_actions::SpecialAction::TurnFaceUp {
                permanent_id: *creature_id,
            };
            crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                .map_err(|e| {
                    GameLoopError::InvalidState(format!("Cannot turn face up: {:?}", e))
                })?;
            crate::special_actions::perform(action, game, player, &mut *decision_maker).map_err(
                |e| GameLoopError::InvalidState(format!("Failed to turn face up: {:?}", e)),
            )?;
            drain_pending_trigger_events(game, trigger_queue);

            // Player retains priority
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::SpecialAction(special) => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            if crate::special_actions::can_perform(special, game, player, &mut *decision_maker)
                .is_ok()
            {
                crate::special_actions::perform(
                    special.clone(),
                    game,
                    player,
                    &mut *decision_maker,
                )
                .map_err(|e| {
                    GameLoopError::InvalidState(format!("Failed special action: {:?}", e))
                })?;
                if let crate::special_actions::SpecialAction::ActivateManaAbility {
                    permanent_id,
                    ..
                } = special
                {
                    queue_ability_activated_event(
                        game,
                        trigger_queue,
                        *permanent_id,
                        player,
                        true,
                        None,
                    );
                }
            }

            // Player retains priority after special actions
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
    }
}

/// Apply a replacement effect choice response.
///
/// When multiple replacement effects could apply to the same event,
/// the affected player must choose which one to apply first.
fn apply_replacement_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    chosen_index: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::event_processor::{TraitEventResult, process_event_with_chosen_replacement_trait};

    // Take the pending choice
    let pending = game
        .pending_replacement_choice
        .take()
        .ok_or_else(|| GameLoopError::InvalidState("No pending replacement choice".to_string()))?;

    // Get the chosen effect ID
    let chosen_id = pending
        .applicable_effects
        .get(chosen_index)
        .copied()
        .unwrap_or_else(|| {
            // Default to first if index is invalid
            pending.applicable_effects[0]
        });

    // Process the event with the chosen replacement effect
    let result = process_event_with_chosen_replacement_trait(game, pending.event, chosen_id);

    // Handle the result
    match result {
        TraitEventResult::Prevented => {
            // Event was prevented - nothing more to do
        }
        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
            // Event can proceed - the actual event application happens
            // at the point where the event was originally generated
            // (e.g., damage application, zone change, etc.)
            // The result is now stored and will be picked up by the caller
        }
        TraitEventResult::Replaced { effects, effect_id } => {
            // Event was replaced with different effects - execute them
            // Consume one-shot effects
            game.replacement_effects.mark_effect_used(effect_id);

            // Get the source/controller from the chosen replacement effect
            let (source, controller) = game
                .replacement_effects
                .get_effect(chosen_id)
                .map(|e| (e.source, e.controller))
                .unwrap_or((ObjectId::from_raw(0), PlayerId::from_index(0)));

            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, controller, &mut dm);

            for effect in effects {
                // Execute each replacement effect
                let _ = execute_effect(game, &effect, &mut ctx);
            }
        }
        TraitEventResult::NeedsChoice {
            player,
            applicable_effects,
            event,
        } => {
            // Build options first (before moving applicable_effects)
            let options: Vec<_> = applicable_effects
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    game.replacement_effects.get_effect(*id).map(|e| {
                        crate::decision::ReplacementOption {
                            index: i,
                            source: e.source,
                            description: format!("{:?}", e.replacement),
                        }
                    })
                })
                .collect();

            // Still more choices needed - store and prompt again
            game.pending_replacement_choice = Some(crate::game_state::PendingReplacementChoice {
                event: *event,
                applicable_effects,
                player,
            });

            // Return to prompt for the next choice - convert to SelectOptionsContext
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();
            let ctx = crate::decisions::context::SelectOptionsContext::new(
                player,
                None,
                "Choose replacement effect to apply",
                selectable_options,
                1,
                1,
            );
            return Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ));
        }
        TraitEventResult::NeedsInteraction { .. } => {
            // Interactive replacements are handled in resolve_stack_entry_full,
            // not in the replacement choice flow
            // This shouldn't happen here, but just proceed if it does
        }
    }

    // Continue with normal game flow
    advance_priority_with_dm(game, trigger_queue, decision_maker)
}

/// Apply a Targets response for a pending spell cast.
fn apply_targets_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    targets: &[Target],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending activation first
    if let Some(mut pending) = state.pending_activation.take() {
        // Combine previously chosen targets with new ones
        pending.chosen_targets.extend(targets.iter().cloned());
        pending.remaining_requirements.clear();

        // Move to next stage
        if pending.mana_cost_to_pay.is_some() {
            pending.stage = ActivationStage::PayingMana;
        } else {
            pending.stage = ActivationStage::ReadyToFinalize;
        }

        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for targets response".to_string())
    })?;

    // Combine previously chosen targets with new ones
    let mut all_targets = pending.chosen_targets.clone();
    all_targets.extend(targets.iter().cloned());

    // Continue to mana payment stage
    continue_to_mana_payment(
        game,
        trigger_queue,
        state,
        pending,
        all_targets,
        decision_maker,
    )
}

/// Apply an X value response for a pending spell cast.
fn apply_x_value_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    x_value: u32,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending activation first
    if let Some(mut pending) = state.pending_activation.take() {
        // Store the X value
        pending.x_value = Some(x_value as usize);

        // Move to next stage (per MTG rule 602.2b, follows 601.2b-h order)
        // After X: Sacrifice → Hybrid/Phyrexian announcement → Targets → Mana payment
        if !pending.remaining_sacrifice_costs.is_empty() {
            pending.stage = ActivationStage::ChoosingSacrifice;
        } else if !pending.pending_hybrid_pips.is_empty() {
            // Hybrid pips were populated at activation start
            pending.stage = ActivationStage::AnnouncingCost;
        } else if pending.hybrid_choices.is_empty() {
            // Check for hybrid pips now (in case X value changed the cost calculation)
            if let Some(ref mana_cost) = pending.mana_cost_to_pay {
                let pips_to_announce = get_pips_requiring_announcement(mana_cost);
                if !pips_to_announce.is_empty() {
                    pending.pending_hybrid_pips = pips_to_announce;
                    pending.stage = ActivationStage::AnnouncingCost;
                    return continue_activation(
                        game,
                        trigger_queue,
                        state,
                        pending,
                        decision_maker,
                    );
                }
            }
            // No hybrid pips, continue to targets
            if !pending.remaining_requirements.is_empty() {
                pending.stage = ActivationStage::ChoosingTargets;
            } else if pending.mana_cost_to_pay.is_some() {
                pending.stage = ActivationStage::PayingMana;
            } else {
                pending.stage = ActivationStage::ReadyToFinalize;
            }
        } else if !pending.remaining_requirements.is_empty() {
            pending.stage = ActivationStage::ChoosingTargets;
        } else if pending.mana_cost_to_pay.is_some() {
            pending.stage = ActivationStage::PayingMana;
        } else {
            pending.stage = ActivationStage::ReadyToFinalize;
        }

        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    // Otherwise handle pending cast
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState(
            "No pending cast or activation for X value response".to_string(),
        )
    })?;

    // Store the X value
    pending.x_value = Some(x_value);

    // Check for optional costs, then continue to targeting or finalization
    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Collect all available casting methods for a spell.
/// Returns a list of CastingMethodOption structs for each method that can be used.
fn collect_available_casting_methods(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    from_zone: Zone,
) -> Vec<crate::decision::CastingMethodOption> {
    use crate::decision::{
        CastingMethodOption, can_cast_spell, can_cast_with_alternative_from_hand,
    };

    let mut methods = Vec::new();

    let Some(spell) = game.object(spell_id) else {
        return methods;
    };

    // Check normal casting method
    if from_zone == Zone::Hand && can_cast_spell(game, player, spell, &CastingMethod::Normal) {
        let cost_desc = spell
            .mana_cost
            .as_ref()
            .map(format_mana_cost_simple)
            .unwrap_or_else(|| "0".to_string());
        methods.push(CastingMethodOption {
            method: CastingMethod::Normal,
            name: "Normal".to_string(),
            cost_description: cost_desc,
        });
    }

    // Check alternative casting methods from hand
    if from_zone == Zone::Hand {
        for (idx, alt_cast) in spell.alternative_casts.iter().enumerate() {
            if alt_cast.cast_from_zone() == Zone::Hand
                && can_cast_with_alternative_from_hand(game, player, spell, spell_id, alt_cast)
            {
                let (name, cost_desc) = format_alternative_method(alt_cast, spell);
                methods.push(CastingMethodOption {
                    method: CastingMethod::Alternative(idx),
                    name,
                    cost_description: cost_desc,
                });
            }
        }
    }

    methods
}

/// Format a mana cost in simple text form (e.g., "{3}{U}{U}").
fn format_mana_cost_simple(cost: &crate::mana::ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            parts.push(match &pip[0] {
                ManaSymbol::Generic(n) => format!("{{{}}}", n),
                ManaSymbol::Colorless => "{C}".to_string(),
                ManaSymbol::White => "{W}".to_string(),
                ManaSymbol::Blue => "{U}".to_string(),
                ManaSymbol::Black => "{B}".to_string(),
                ManaSymbol::Red => "{R}".to_string(),
                ManaSymbol::Green => "{G}".to_string(),
                ManaSymbol::Snow => "{S}".to_string(),
                ManaSymbol::X => "{X}".to_string(),
                ManaSymbol::Life(n) => format!("{{{}/P}}", n),
            });
        } else {
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::X => "X".to_string(),
                    ManaSymbol::Life(n) => format!("P/{}", n),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("")
    }
}

/// Format an alternative casting method's name and cost description.
fn format_alternative_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    spell: &crate::object::Object,
) -> (String, String) {
    use crate::alternative_cast::AlternativeCastingMethod;

    match method {
        AlternativeCastingMethod::Flashback { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Flashback".to_string(), cost_desc)
        }
        AlternativeCastingMethod::JumpStart => {
            // Jump-start uses the spell's mana cost plus discarding a card
            let cost_desc = spell
                .mana_cost
                .as_ref()
                .map(format_mana_cost_simple)
                .unwrap_or_else(|| "0".to_string());
            (
                "Jump-Start".to_string(),
                format!("{}, Discard a card", cost_desc),
            )
        }
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            let cost_desc = cost
                .as_ref()
                .map(format_mana_cost_simple)
                .or_else(|| spell.mana_cost.as_ref().map(format_mana_cost_simple))
                .unwrap_or_else(|| "0".to_string());
            (
                "Escape".to_string(),
                format!("{}, Exile {} cards from graveyard", cost_desc, exile_count),
            )
        }
        AlternativeCastingMethod::Composed { .. } => {
            let mana_cost = method.mana_cost();
            let cost_effects = method.cost_effects();
            let name = method.name();
            let mut parts = Vec::new();
            if let Some(mana) = mana_cost {
                parts.push(format_mana_cost_simple(mana));
            }
            for effect in cost_effects {
                parts.push(format!("{:?}", effect));
            }
            let cost_desc = if parts.is_empty() {
                "Free".to_string()
            } else {
                parts.join(", ")
            };
            (name.to_string(), cost_desc)
        }
        AlternativeCastingMethod::MindbreakTrap {
            cost, condition, ..
        } => {
            let cost_desc = format_mana_cost_simple(cost);
            let condition_desc = match condition {
                crate::alternative_cast::TrapCondition::OpponentCastSpells { count } => {
                    format!("If opponent cast {}+ spells this turn", count)
                }
                crate::alternative_cast::TrapCondition::OpponentSearchedLibrary => {
                    "If opponent searched their library".to_string()
                }
                crate::alternative_cast::TrapCondition::OpponentCreatureEntered => {
                    "If opponent had a creature enter".to_string()
                }
                crate::alternative_cast::TrapCondition::CreatureDealtDamageToYou => {
                    "If a creature dealt damage to you".to_string()
                }
            };
            (
                "Trap".to_string(),
                format!("{} ({})", cost_desc, condition_desc),
            )
        }
        AlternativeCastingMethod::Madness { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Madness".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Miracle { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Miracle".to_string(), cost_desc)
        }
    }
}

/// Helper to extract modal spec from a spell's effects.
///
/// Searches through the spell's effects to find if it has a modal effect (ChooseModeEffect).
/// For compositional effects like ConditionalEffect, this evaluates conditions at cast time
/// to determine which branch's modal spec to use (e.g., Akroma's Will checking YouControlCommander).
/// Returns the modal specification if found.
fn extract_modal_spec_from_spell(
    game: &GameState,
    spell_id: ObjectId,
    controller: PlayerId,
) -> Option<crate::effects::ModalSpec> {
    let obj = game.object(spell_id)?;

    // Check spell effects with context to handle conditional effects like Akroma's Will
    if let Some(ref effects) = obj.spell_effect {
        for effect in effects {
            // Try context-aware extraction first (handles ConditionalEffect)
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, spell_id)
            {
                return Some(spec);
            }
            // Fall back to simple extraction for direct modal effects
            if let Some(spec) = effect.0.get_modal_spec() {
                return Some(spec);
            }
        }
    }

    None
}

/// Check for modal effects and either prompt for mode selection or continue to optional costs.
///
/// Per MTG rule 601.2b, modes must be chosen before targets.
/// This is called after the spell is proposed (moved to stack).
fn check_modes_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if the spell has modal effects (with context for conditional effects like Akroma's Will)
    if let Some(modal_spec) = extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
    {
        let player = pending.caster;
        let source = pending.spell_id;

        // Resolve min/max mode counts
        let max_modes = match &modal_spec.max_modes {
            crate::effect::Value::Fixed(n) => *n as usize,
            _ => 1, // Default to 1 for dynamic values during casting
        };
        let min_modes = match &modal_spec.min_modes {
            crate::effect::Value::Fixed(n) => *n as usize,
            _ => max_modes, // Default to max for exact choice
        };

        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        // Set up pending cast for modes stage
        let mut pending = pending;
        pending.stage = CastStage::ChoosingModes;
        state.pending_cast = Some(pending);

        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Modes(
                crate::decisions::context::ModesContext {
                    player,
                    source: Some(source),
                    spell_name,
                    spec: crate::decisions::ModesSpec::new(
                        source,
                        modal_spec
                            .mode_descriptions
                            .iter()
                            .enumerate()
                            .map(|(i, desc)| {
                                crate::decisions::specs::ModeOption::with_legality(
                                    i,
                                    desc.clone(),
                                    true,
                                )
                            })
                            .collect(),
                        min_modes,
                        max_modes,
                    ),
                },
            ),
        ))
    } else {
        // No modal effects, continue to optional costs
        check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Check for optional costs and either prompt for them or continue to targeting/finalization.
///
/// This is called after X value is chosen (or when there's no X cost).
/// Returns the next decision needed or continues the cast.
fn check_optional_costs_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if the spell has optional costs
    let optional_costs = if let Some(obj) = game.object(pending.spell_id) {
        obj.optional_costs.clone()
    } else {
        Vec::new()
    };

    if optional_costs.is_empty() {
        // No optional costs, continue to targeting or finalization
        continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Build the optional cost options for the decision
        let player = pending.caster;
        let source = pending.spell_id;

        // Check which costs the player can afford (using potential mana)
        let options: Vec<OptionalCostOption> = optional_costs
            .iter()
            .enumerate()
            .map(|(index, opt_cost)| {
                // Check if player can afford this cost with potential mana
                let affordable = if let Some(mana_cost) = opt_cost.cost.mana_cost() {
                    crate::decision::can_potentially_pay(game, player, mana_cost, 0)
                } else {
                    // For non-mana costs, use the regular check
                    crate::cost::can_pay_cost(game, source, player, &opt_cost.cost).is_ok()
                };

                // Format the cost description
                let cost_description = if let Some(mana) = opt_cost.cost.mana_cost() {
                    format!("{}", mana.mana_value())
                } else {
                    "special".to_string()
                };

                OptionalCostOption {
                    index,
                    label: opt_cost.label,
                    repeatable: opt_cost.repeatable,
                    affordable,
                    cost_description,
                }
            })
            .collect();

        // Set up pending cast for optional costs stage
        let mut pending = pending;
        pending.stage = CastStage::ChoosingOptionalCosts;
        state.pending_cast = Some(pending);

        // Convert to SelectOptionsContext for optional cost selection
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::with_legality(
                    opt.index,
                    format!("{}: {}", opt.label, opt.cost_description),
                    opt.affordable,
                )
            })
            .collect();
        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            player,
            Some(source),
            format!("Choose optional costs for {}", spell_name),
            selectable_options,
            0,             // min - optional costs are optional
            options.len(), // max - can select all
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ))
    }
}

/// Get the effective mana cost for a spell being cast.
///
/// This is called during casting to determine hybrid/Phyrexian pips.
fn get_spell_mana_cost(
    game: &GameState,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
) -> Option<crate::mana::ManaCost> {
    let obj = game.object(spell_id)?;
    match casting_method {
        CastingMethod::Normal => obj.mana_cost.clone(),
        CastingMethod::Alternative(idx) => {
            if let Some(method) = obj.alternative_casts.get(*idx) {
                // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                // For other methods (flashback, etc.), fall back to spell's cost.
                if !method.cost_effects().is_empty() {
                    method.mana_cost().cloned()
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| obj.mana_cost.clone())
                }
            } else {
                obj.mana_cost.clone()
            }
        }
        CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
        CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => obj.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            ..
        } => {
            if let Some(method) = obj.alternative_casts.get(*idx) {
                // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                // For other methods (flashback, etc.), fall back to spell's cost
                if !method.cost_effects().is_empty() {
                    method.mana_cost().cloned()
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| obj.mana_cost.clone())
                }
            } else {
                obj.mana_cost.clone()
            }
        }
    }
}

/// Get pips that require announcement (hybrid/Phyrexian pips with multiple options).
///
/// Returns a list of (pip_index, alternatives) for each pip that has multiple payment options.
/// Per MTG rule 601.2b, the player must announce how they will pay these during casting.
fn get_pips_requiring_announcement(
    cost: &crate::mana::ManaCost,
) -> Vec<(usize, Vec<crate::mana::ManaSymbol>)> {
    cost.pips()
        .iter()
        .enumerate()
        .filter(|(_, pip)| pip.len() > 1) // Multiple options = needs choice
        .map(|(i, pip)| (i, pip.clone()))
        .collect()
}

/// Continue the casting process to targeting or mana payment.
///
/// Called when there are no optional costs or after optional costs are chosen.
/// Per MTG rule 601.2b, checks for hybrid/Phyrexian pips first.
fn continue_to_targeting_or_finalize(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Per MTG 601.2b: Check for hybrid/Phyrexian pips that need announcement BEFORE targets
    // Skip if we already have hybrid choices (coming back from AnnouncingCost stage)
    if pending.hybrid_choices.is_empty()
        && let Some(mana_cost) =
            get_spell_mana_cost(game, pending.spell_id, &pending.casting_method)
    {
        let pips_to_announce = get_pips_requiring_announcement(&mana_cost);
        if !pips_to_announce.is_empty() {
            // Need to announce hybrid/Phyrexian choices
            return check_hybrid_announcement_or_continue(
                game,
                trigger_queue,
                state,
                pending,
                pips_to_announce,
                decision_maker,
            );
        }
    }

    // No hybrid/Phyrexian pips (or already announced), continue to targets
    continue_to_targets_or_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Check for hybrid/Phyrexian pips and prompt for announcements.
///
/// Per MTG rule 601.2b, the player announces how they will pay hybrid/Phyrexian costs
/// before targets are chosen.
fn check_hybrid_announcement_or_continue(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    pips_to_announce: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = pending;
    pending.stage = CastStage::AnnouncingCost;
    pending.pending_hybrid_pips = pips_to_announce;

    // Prompt for the first pip
    prompt_for_next_hybrid_pip(game, state, pending)
}

/// Prompt the player for the next hybrid/Phyrexian pip choice.
fn prompt_for_next_hybrid_pip(
    game: &GameState,
    state: &mut PriorityLoopState,
    pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    // Get the next pip to announce
    if let Some((pip_idx, alternatives)) = pending.pending_hybrid_pips.first().cloned() {
        let player = pending.caster;
        let source = pending.spell_id;
        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        // Build hybrid options for each alternative
        let options: Vec<crate::decisions::context::HybridOption> = alternatives
            .iter()
            .enumerate()
            .map(|(i, sym)| crate::decisions::context::HybridOption {
                index: i,
                label: format_mana_symbol_for_choice(sym),
                symbol: *sym,
            })
            .collect();

        state.pending_cast = Some(pending);

        // Create a HybridChoice decision for this pip
        let ctx = crate::decisions::context::HybridChoiceContext::new(
            player,
            Some(source),
            spell_name,
            pip_idx + 1, // 1-based for display
            options,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::HybridChoice(ctx),
        ))
    } else {
        // No more pips to announce - this shouldn't happen, but handle gracefully
        state.pending_cast = Some(pending);
        Err(GameLoopError::InvalidState(
            "No pending hybrid pips to announce".to_string(),
        ))
    }
}

/// Format a mana symbol for display in hybrid/Phyrexian choice.
fn format_mana_symbol_for_choice(sym: &crate::mana::ManaSymbol) -> String {
    use crate::mana::ManaSymbol;
    match sym {
        ManaSymbol::White => "{W} (White mana)".to_string(),
        ManaSymbol::Blue => "{U} (Blue mana)".to_string(),
        ManaSymbol::Black => "{B} (Black mana)".to_string(),
        ManaSymbol::Red => "{R} (Red mana)".to_string(),
        ManaSymbol::Green => "{G} (Green mana)".to_string(),
        ManaSymbol::Colorless => "{C} (Colorless mana)".to_string(),
        ManaSymbol::Generic(n) => format!("{{{}}} ({} generic mana)", n, n),
        ManaSymbol::Snow => "{S} (Snow mana)".to_string(),
        ManaSymbol::Life(n) => format!("{} life (Phyrexian)", n),
        ManaSymbol::X => "{X}".to_string(),
    }
}

/// Continue to target selection or mana payment.
///
/// Called after hybrid/Phyrexian choices are made (or when none are needed).
fn continue_to_targets_or_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Validate that we can still pay the cost after hybrid choices
    // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
    // but the player may have chosen mana payment instead
    if let Some(ref cost) = pending.mana_cost_to_pay {
        let x_value = pending.x_value.unwrap_or(0);
        let expanded_pips =
            expand_mana_cost_to_pips(cost, x_value as usize, &pending.hybrid_choices);
        let potential = compute_potential_mana(game, pending.caster);

        // Check if we can pay all the expanded pips (excluding life payments)
        let total_mana_needed: usize = expanded_pips
            .iter()
            .filter(|pip| {
                !pip.iter()
                    .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
            })
            .count();

        if potential.total() < total_mana_needed as u32 {
            return Err(GameLoopError::InvalidState(format!(
                "Cannot afford spell: need {} mana but only have {} available. \
                Consider paying life for Phyrexian mana or choosing a lower X value.",
                total_mana_needed,
                potential.total()
            )));
        }
    }

    if pending.remaining_requirements.is_empty() {
        // No targets needed, go to mana payment
        continue_to_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            Vec::new(),
            decision_maker,
        )
    } else {
        // Need to select targets
        let mut pending = pending;
        pending.stage = CastStage::ChoosingTargets;
        let requirements = pending.remaining_requirements.clone();
        let player = pending.caster;
        let source = pending.spell_id;
        let context = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        state.pending_cast = Some(pending);

        // Convert to TargetsContext
        let ctx = crate::decisions::context::TargetsContext::new(
            player,
            source,
            context,
            requirements
                .into_iter()
                .map(|r| crate::decisions::context::TargetRequirementContext {
                    description: r.description,
                    legal_targets: r.legal_targets,
                    min_targets: r.min_targets,
                    max_targets: r.max_targets,
                })
                .collect(),
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        ))
    }
}

/// Continue the casting process to mana payment stage.
///
/// Called after targets are chosen (or when no targets needed).
/// Computes the effective mana cost and checks if payment is possible.
fn continue_to_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    targets: Vec<Target>,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_for_payment_with_targets;

    let mut pending = pending;
    pending.chosen_targets = targets;

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
        // Get base cost from casting method
        let base_cost = match &pending.casting_method {
            CastingMethod::Normal => obj.mana_cost.clone(),
            CastingMethod::Alternative(idx) => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
            CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
            CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                ..
            } => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
        };

        // Apply cost reductions (affinity, delve, convoke, improvise)
        base_cost.map(|bc| {
            calculate_effective_mana_cost_for_payment_with_targets(
                game,
                pending.caster,
                obj,
                &bc,
                pending.chosen_targets.len(),
            )
        })
    } else {
        None
    };

    pending.mana_cost_to_pay = effective_cost.clone();

    // Check for ExileFromHand costs that need player choice
    let exile_from_hand_choice_needed =
        if let CastingMethod::Alternative(idx) = &pending.casting_method {
            if let Some(obj) = game.object(pending.spell_id) {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // Check for exile from hand requirement in cost_effects
                    if let Some((count, color_filter)) = method.exile_from_hand_requirement() {
                        // Check if there are multiple legal cards to choose from
                        if let Some(player) = game.player(pending.caster) {
                            let matching_cards: Vec<ObjectId> = player
                                .hand
                                .iter()
                                .filter(|&&card_id| {
                                    if card_id == pending.spell_id {
                                        return false;
                                    }
                                    if let Some(filter) = color_filter {
                                        if let Some(card) = game.object(card_id) {
                                            let card_colors = card.colors();
                                            !card_colors.intersection(filter).is_empty()
                                        } else {
                                            false
                                        }
                                    } else {
                                        true
                                    }
                                })
                                .copied()
                                .collect();
                            // Need choice if there are more cards than required
                            matching_cards.len() > count as usize
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

    // If we need to choose cards to exile, prompt for that first
    if exile_from_hand_choice_needed && pending.cards_to_exile.is_empty() {
        // Find the legal cards to exile
        let legal_cards = if let CastingMethod::Alternative(idx) = &pending.casting_method {
            if let Some(obj) = game.object(pending.spell_id) {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    if let Some((_, color_filter)) = method.exile_from_hand_requirement() {
                        let mut cards = Vec::new();
                        if let Some(player) = game.player(pending.caster) {
                            for &card_id in &player.hand {
                                if card_id == pending.spell_id {
                                    continue;
                                }
                                let matches = if let Some(filter) = color_filter {
                                    if let Some(card) = game.object(card_id) {
                                        let card_colors = card.colors();
                                        !card_colors.intersection(filter).is_empty()
                                    } else {
                                        false
                                    }
                                } else {
                                    true
                                };
                                if matches {
                                    cards.push(card_id);
                                }
                            }
                        }
                        cards
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        pending.stage = CastStage::ChoosingExileFromHand;
        let player = pending.caster;
        let source = pending.spell_id;
        state.pending_cast = Some(pending);

        // Convert to SelectObjectsContext for card to exile selection
        let candidates: Vec<crate::decisions::context::SelectableObject> = legal_cards
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| format!("Card #{}", id.0));
                crate::decisions::context::SelectableObject::new(id, name)
            })
            .collect();
        let ctx = crate::decisions::context::SelectObjectsContext::new(
            player,
            Some(source),
            "Exile a blue card from your hand",
            candidates,
            1,
            Some(1),
        );
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(ctx),
        ));
    }

    pending.stage = CastStage::PayingMana;

    // Store the mana cost in pending for pip-by-pip processing
    pending.mana_cost_to_pay = effective_cost;

    // Continue with pip-by-pip mana payment
    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Continue processing spell cast mana payment pip-by-pip.
fn continue_spell_cast_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let x_value = pending.x_value.unwrap_or(0);

    // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
    // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
    if pending.remaining_mana_pips.is_empty()
        && let Some(cost) = pending.mana_cost_to_pay.take()
    {
        pending.remaining_mana_pips =
            expand_mana_cost_to_pips(&cost, x_value as usize, &pending.hybrid_choices);
    }

    // If no remaining pips, we're done with mana payment - finalize the spell
    if pending.remaining_mana_pips.is_empty() {
        let mana_spent_to_cast = pending.mana_spent_to_cast.clone();
        let result = finalize_spell_cast(
            game,
            trigger_queue,
            state,
            pending.spell_id,
            pending.from_zone,
            pending.caster,
            pending.chosen_targets,
            pending.x_value,
            pending.casting_method,
            pending.optional_costs_paid,
            pending.chosen_modes,
            pending.cards_to_exile,
            mana_spent_to_cast,
            pending.keyword_payment_contributions,
            &mut pending.payment_trace,
            true, // mana_already_paid via pip-by-pip
            pending.stack_id,
            &mut *decision_maker,
        )?;

        // Generate SpellCast event and check for triggers
        let event = TriggerEvent::new(SpellCastEvent::new(
            result.new_id,
            result.caster,
            result.from_zone,
        ));
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }

        // Clear checkpoint - spell cast completed successfully
        state.clear_checkpoint();
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    // Get the first pip to pay
    let pip = pending.remaining_mana_pips[0].clone();
    let remaining_count = pending.remaining_mana_pips.len() - 1;

    // Build payment options for this pip
    let player_id = pending.caster;
    let source = pending.spell_id;
    let context = game
        .object(source)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| "spell".to_string());

    let allow_any_color = game.can_spend_mana_as_any_color(player_id, Some(source));
    let options = build_pip_payment_options(
        game,
        player_id,
        &pip,
        allow_any_color,
        Some(source),
        &mut *decision_maker,
    );

    // If no options available (shouldn't happen if we validated correctly), error
    if options.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No payment options available for mana pip".to_string(),
        ));
    }

    // Auto-select deterministic pip choices when possible.
    if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
        let action = options[auto_choice].action.clone();
        let pip_paid = execute_pip_payment_action(
            game,
            trigger_queue,
            player_id,
            Some(source),
            &action,
            &mut *decision_maker,
            &mut pending.payment_trace,
            Some(&mut pending.mana_spent_to_cast),
        )?;
        queue_mana_ability_event_for_action(game, trigger_queue, &action, player_id);
        drain_pending_trigger_events(game, trigger_queue);
        if pip_paid {
            record_keyword_payment_contribution(
                &mut pending.keyword_payment_contributions,
                &action,
            );
            pending.remaining_mana_pips.remove(0);
        }
        return continue_spell_cast_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let pip_description = format_pip(&pip);

    state.pending_cast = Some(pending);

    // Convert ManaPipPaymentOption to SelectableOption
    let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
        .iter()
        .map(|opt| crate::decisions::context::SelectableOption::new(opt.index, &opt.description))
        .collect();

    let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
        player_id,
        source,
        context,
        pip_description,
        remaining_count,
        selectable_options,
    );
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectOptions(ctx),
    ))
}

/// Compute available mana payment options for a player during mana ability activation.
///
/// This returns options for:
/// - Available mana abilities that can be activated (excluding the one being paid for)
///   and that can help pay the remaining cost
/// - Option to pay (if enough mana is in pool)
fn compute_mana_ability_payment_options(
    game: &GameState,
    player: PlayerId,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPaymentOption> {
    use crate::ability::AbilityKind;

    let mut options = Vec::new();

    // Get available mana abilities the player can activate
    // Exclude the mana ability we're trying to pay for
    let mana_abilities = get_available_mana_abilities(game, player, decision_maker);

    // Filter to only abilities that can help pay the cost
    let mut option_index = 0;
    for (perm_id, ability_index, description) in mana_abilities.iter() {
        // Skip the ability we're trying to pay for to avoid infinite loop
        if *perm_id == pending.source && *ability_index == pending.ability_index {
            continue;
        }

        // Get the mana this ability produces and check if it can help pay the cost
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(pending.source));
        let can_help = if let Some(perm) = game.object(*perm_id)
            && let Some(ability) = perm.abilities.get(*ability_index)
            && let AbilityKind::Mana(mana_ability) = &ability.kind
        {
            mana_can_help_pay_cost(
                &mana_ability.mana,
                &pending.mana_cost,
                game,
                player,
                allow_any_color,
            )
        } else {
            // If we can't determine, include it
            true
        };

        if can_help {
            options.push(ManaPaymentOption {
                index: option_index,
                description: format!(
                    "Tap {}: {}",
                    describe_permanent(game, *perm_id),
                    description
                ),
            });
            option_index += 1;
        }
    }

    // Add option to pay if player has enough mana
    if game.can_pay_mana_cost(player, Some(pending.source), &pending.mana_cost, 0) {
        options.push(ManaPaymentOption {
            index: options.len(),
            description: "Pay mana cost".to_string(),
        });
    }

    options
}

/// Check if mana produced by an ability can help pay a mana cost.
///
/// Returns true if any of the mana symbols can pay any pip in the cost,
/// considering the player's current mana pool.
fn mana_can_help_pay_cost(
    mana_produced: &[crate::mana::ManaSymbol],
    cost: &crate::mana::ManaCost,
    game: &GameState,
    player: PlayerId,
    allow_any_color: bool,
) -> bool {
    use crate::mana::ManaSymbol;

    // Get current mana pool to see what's already available
    let pool = game.player(player).map(|p| &p.mana_pool);

    // Check each pip in the cost to see if the produced mana can help
    for pip in cost.pips() {
        for alternative in pip {
            match alternative {
                // Generic mana can be paid by any colored mana
                ManaSymbol::Generic(_) => {
                    // Any mana helps with generic costs
                    if !mana_produced.is_empty() {
                        return true;
                    }
                }
                // Colored mana must match
                ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green => {
                    // If any-color spending is allowed, any mana helps with colored pips
                    if allow_any_color {
                        if !mana_produced.is_empty() {
                            return true;
                        }
                    } else if mana_produced.contains(alternative) {
                        return true;
                    }
                }
                // Colorless mana can only be paid by colorless
                ManaSymbol::Colorless => {
                    if mana_produced.contains(&ManaSymbol::Colorless) {
                        return true;
                    }
                }
                // Snow, life, X - less common, be permissive
                _ => return true,
            }
        }
    }

    // Also check if this mana could help after we pay some colored pips
    // (e.g., we might need {W}{W} and only have one white, so any mana helps with the first)
    // For simplicity, if the cost has any generic component that's not yet payable, any mana helps
    if pool.is_some() {
        let generic_needed = cost
            .pips()
            .iter()
            .filter(|pip| pip.iter().any(|s| matches!(s, ManaSymbol::Generic(_))))
            .count();

        // Very rough heuristic: if there are generic costs and the ability produces any mana
        if generic_needed > 0 && !mana_produced.is_empty() {
            return true;
        }
    }

    false
}

/// Get available mana abilities for a player that can be activated.
///
/// Returns a list of (permanent_id, ability_index, description) tuples.
fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<(ObjectId, usize, String)> {
    use crate::ability::AbilityKind;
    use crate::special_actions::{SpecialAction, can_perform};

    let mut abilities = Vec::new();

    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        if perm.controller != player {
            continue;
        }

        for (i, ability) in perm.abilities.iter().enumerate() {
            if matches!(ability.kind, AbilityKind::Mana(_)) {
                let action = SpecialAction::ActivateManaAbility {
                    permanent_id: perm_id,
                    ability_index: i,
                };

                if can_perform(&action, game, player, &mut *decision_maker).is_ok() {
                    let desc = describe_mana_ability(&ability.kind);
                    abilities.push((perm_id, i, desc));
                }
            }
        }
    }

    abilities
}

/// Describe a mana ability for display.
fn describe_mana_ability(kind: &crate::ability::AbilityKind) -> String {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    if let AbilityKind::Mana(mana_ability) = kind {
        let mana_strs: Vec<&str> = mana_ability
            .mana
            .iter()
            .map(|m| match m {
                ManaSymbol::White => "{W}",
                ManaSymbol::Blue => "{U}",
                ManaSymbol::Black => "{B}",
                ManaSymbol::Red => "{R}",
                ManaSymbol::Green => "{G}",
                ManaSymbol::Colorless => "{C}",
                _ => "mana",
            })
            .collect();
        format!("Add {}", mana_strs.join(""))
    } else {
        "Add mana".to_string()
    }
}

/// Describe a permanent for display.
fn describe_permanent(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|obj| obj.name.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Get legal sacrifice targets for a filter.
fn get_legal_sacrifice_targets(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    filter: &PermanentFilter,
) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .filter(|&&id| {
            let Some(obj) = game.object(id) else {
                return false;
            };

            // Must be controlled by player
            if obj.controller != player {
                return false;
            }

            // Check "other" requirement
            if filter.other && id == source {
                return false;
            }

            // Check card type filter
            if !filter.card_types.is_empty()
                && !filter.card_types.iter().any(|t| obj.has_card_type(*t))
            {
                return false;
            }

            // Check subtype filter
            if !filter.subtypes.is_empty()
                && !filter.subtypes.iter().any(|s| obj.subtypes.contains(s))
            {
                return false;
            }

            // Check token/nontoken
            if filter.token && obj.kind != crate::object::ObjectKind::Token {
                return false;
            }
            if filter.nontoken && obj.kind == crate::object::ObjectKind::Token {
                return false;
            }

            true
        })
        .copied()
        .collect()
}

/// Continue the activation process based on current stage.
fn continue_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Re-check activation constraints while continuing an in-flight activation.
    let Some(obj) = game.object(pending.source) else {
        return Err(GameLoopError::InvalidState(
            "Ability source no longer exists".to_string(),
        ));
    };
    let Some(ability) = obj.abilities.get(pending.ability_index) else {
        return Err(GameLoopError::InvalidState(
            "Ability index no longer valid".to_string(),
        ));
    };
    if let AbilityKind::Activated(activated) = &ability.kind {
        if !can_activate_ability_with_restrictions(
            game,
            pending.source,
            pending.ability_index,
            activated,
        ) {
            return Err(GameLoopError::InvalidState(
                "Ability activation restrictions are no longer satisfied".to_string(),
            ));
        }
    } else {
        return Err(GameLoopError::InvalidState(
            "Pending ability is not an activated ability".to_string(),
        ));
    }

    match pending.stage {
        ActivationStage::ChoosingX => {
            // Need to choose X value first
            let max_x = if let Some(ref cost) = pending.mana_cost_to_pay {
                let allow_any_color =
                    game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
                compute_potential_mana(game, pending.activator)
                    .max_x_for_cost_with_any_color(cost, allow_any_color)
            } else {
                0
            };

            state.pending_activation = Some(pending.clone());

            let ctx = crate::decisions::context::NumberContext::x_value(
                pending.activator,
                pending.source,
                max_x,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Number(ctx),
            ))
        }
        ActivationStage::ChoosingSacrifice => {
            // Get the next sacrifice cost to pay
            if let Some((filter, description)) = pending.remaining_sacrifice_costs.first().cloned()
            {
                let legal_targets =
                    get_legal_sacrifice_targets(game, pending.activator, pending.source, &filter);

                if legal_targets.is_empty() {
                    // No valid targets - this shouldn't happen if can_pay_cost was checked
                    return Err(GameLoopError::InvalidState(
                        "No valid sacrifice targets".to_string(),
                    ));
                }

                let player = pending.activator;
                let source = pending.source;
                state.pending_activation = Some(pending);

                // Convert to SelectObjectsContext for sacrifice selection
                let candidates: Vec<crate::decisions::context::SelectableObject> = legal_targets
                    .iter()
                    .map(|&id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| format!("Permanent #{}", id.0));
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect();
                let ctx = crate::decisions::context::SelectObjectsContext::new(
                    player,
                    Some(source),
                    format!("Choose a creature to sacrifice: {}", description),
                    candidates,
                    1,
                    Some(1),
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectObjects(ctx),
                ))
            } else {
                // No more sacrifice costs - recompute target requirements with current game state
                // This ensures sacrificed creatures are no longer in the legal targets list
                pending.remaining_requirements = extract_target_requirements(
                    game,
                    &pending.effects,
                    pending.activator,
                    Some(pending.source),
                );

                // Per MTG rule 602.2b (which references 601.2b), check for hybrid/Phyrexian pips
                // These must be announced BEFORE targets are chosen
                if pending.hybrid_choices.is_empty()
                    && let Some(ref mana_cost) = pending.mana_cost_to_pay
                {
                    let pips_to_announce = get_pips_requiring_announcement(mana_cost);
                    if !pips_to_announce.is_empty() {
                        pending.pending_hybrid_pips = pips_to_announce;
                        pending.stage = ActivationStage::AnnouncingCost;
                        return continue_activation(
                            game,
                            trigger_queue,
                            state,
                            pending,
                            decision_maker,
                        );
                    }
                }

                // Move to next stage
                if !pending.remaining_requirements.is_empty() {
                    pending.stage = ActivationStage::ChoosingTargets;
                } else if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                continue_activation(game, trigger_queue, state, pending, decision_maker)
            }
        }
        ActivationStage::AnnouncingCost => {
            // Handle hybrid/Phyrexian mana announcement (per MTG rule 601.2b via 602.2b)
            if pending.pending_hybrid_pips.is_empty() {
                // All hybrid pips announced - validate that we can still pay the cost
                // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
                // but the player may have chosen mana payment instead
                if let Some(ref cost) = pending.mana_cost_to_pay {
                    let x_value = pending.x_value.unwrap_or(0);
                    let expanded_pips =
                        expand_mana_cost_to_pips(cost, x_value, &pending.hybrid_choices);
                    let potential = compute_potential_mana(game, pending.activator);

                    // Check if we can pay all the expanded pips
                    let total_mana_needed: usize = expanded_pips
                        .iter()
                        .filter(|pip| {
                            !pip.iter()
                                .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
                        })
                        .count();

                    if potential.total() < total_mana_needed as u32 {
                        return Err(GameLoopError::InvalidState(format!(
                            "Cannot afford ability: need {} mana but only have {} available. \
                            Consider paying life for Phyrexian mana or choosing a lower X value.",
                            total_mana_needed,
                            potential.total()
                        )));
                    }
                }

                // All hybrid pips announced, move to targets
                if !pending.remaining_requirements.is_empty() {
                    pending.stage = ActivationStage::ChoosingTargets;
                } else if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            // Prompt for the next hybrid pip
            let (pip_idx, alternatives) = pending.pending_hybrid_pips[0].clone();
            let player = pending.activator;
            let source = pending.source;
            let ability_name = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());

            // Build hybrid options for each alternative
            let options: Vec<crate::decisions::context::HybridOption> = alternatives
                .iter()
                .enumerate()
                .map(|(i, sym)| crate::decisions::context::HybridOption {
                    index: i,
                    label: format_mana_symbol_for_choice(sym),
                    symbol: *sym,
                })
                .collect();

            state.pending_activation = Some(pending);

            // Create a HybridChoice decision for this pip
            let ctx = crate::decisions::context::HybridChoiceContext::new(
                player,
                Some(source),
                ability_name,
                pip_idx + 1, // 1-based for display
                options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::HybridChoice(ctx),
            ))
        }
        ActivationStage::ChoosingTargets => {
            if pending.remaining_requirements.is_empty() {
                // No more targets needed
                if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                continue_activation(game, trigger_queue, state, pending, decision_maker)
            } else {
                let requirements = pending.remaining_requirements.clone();
                let player = pending.activator;
                let source = pending.source;
                let context = game
                    .object(source)
                    .map(|o| format!("{}'s ability", o.name))
                    .unwrap_or_else(|| "ability".to_string());

                state.pending_activation = Some(pending);

                // Convert to TargetsContext
                let ctx = crate::decisions::context::TargetsContext::new(
                    player,
                    source,
                    context,
                    requirements
                        .into_iter()
                        .map(|r| crate::decisions::context::TargetRequirementContext {
                            description: r.description,
                            legal_targets: r.legal_targets,
                            min_targets: r.min_targets,
                            max_targets: r.max_targets,
                        })
                        .collect(),
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(ctx),
                ))
            }
        }
        ActivationStage::PayingMana => {
            let x_value = pending.x_value.unwrap_or(0);

            // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
            // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
            if pending.remaining_mana_pips.is_empty()
                && let Some(cost) = pending.mana_cost_to_pay.take()
            {
                pending.remaining_mana_pips =
                    expand_mana_cost_to_pips(&cost, x_value, &pending.hybrid_choices);
            }

            // If no remaining pips, we're done with mana payment
            if pending.remaining_mana_pips.is_empty() {
                pending.stage = ActivationStage::ReadyToFinalize;
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            // Get the first pip to pay
            let pip = pending.remaining_mana_pips[0].clone();
            let remaining_count = pending.remaining_mana_pips.len() - 1;

            // Build payment options for this pip
            let player_id = pending.activator;
            let source = pending.source;
            let context = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());

            let allow_any_color = game.can_spend_mana_as_any_color(player_id, Some(source));
            let options = build_pip_payment_options(
                game,
                player_id,
                &pip,
                allow_any_color,
                None,
                &mut *decision_maker,
            );

            // If no options available (shouldn't happen if we validated correctly), error
            if options.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No payment options available for mana pip".to_string(),
                ));
            }

            // Auto-select deterministic pip choices when possible.
            if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
                let action = options[auto_choice].action.clone();
                let pip_paid = execute_pip_payment_action(
                    game,
                    trigger_queue,
                    player_id,
                    Some(source),
                    &action,
                    &mut *decision_maker,
                    &mut pending.payment_trace,
                    None,
                )?;
                queue_mana_ability_event_for_action(game, trigger_queue, &action, player_id);
                drain_pending_trigger_events(game, trigger_queue);
                if pip_paid {
                    pending.remaining_mana_pips.remove(0);
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            let pip_description = format_pip(&pip);

            state.pending_activation = Some(pending);

            // Convert ManaPipPaymentOption to SelectableOption
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();

            let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
                player_id,
                source,
                context,
                pip_description,
                remaining_count,
                selectable_options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
        ActivationStage::ReadyToFinalize => {
            // Record activation for per-turn-limited abilities
            if pending.is_once_per_turn {
                game.record_ability_activation(pending.source, pending.ability_index);
            }

            // Create ability stack entry with targets
            let mut entry =
                StackEntry::ability(pending.source, pending.activator, pending.effects.clone())
                    .with_source_info(pending.source_stable_id, pending.source_name.clone())
                    .with_source_snapshot(pending.source_snapshot.clone());
            entry.targets = pending.chosen_targets.clone();

            // Pass X value to stack entry so it's available during resolution
            if let Some(x) = pending.x_value {
                entry = entry.with_x(x as u32);
            }

            game.push_to_stack(entry);
            queue_becomes_targeted_events(
                game,
                trigger_queue,
                &pending.chosen_targets,
                pending.source,
                pending.activator,
                true,
            );
            queue_ability_activated_event(
                game,
                trigger_queue,
                pending.source,
                pending.activator,
                false,
                Some(pending.source_stable_id),
            );

            // Clear pending state and checkpoint - action completed successfully
            state.pending_activation = None;
            state.clear_checkpoint();
            reset_priority(game, &mut state.tracker);
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
    }
}

// ============================================================================
// Pip-by-Pip Mana Payment Helpers
// ============================================================================

/// Expand a ManaCost into individual pips, expanding X pips by the chosen value.
/// Also applies hybrid_choices to replace multi-symbol pips with the chosen symbol.
fn expand_mana_cost_to_pips(
    cost: &crate::mana::ManaCost,
    x_value: usize,
    hybrid_choices: &[(usize, crate::mana::ManaSymbol)],
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let mut colored_pips = Vec::new();
    let mut generic_pips = Vec::new();

    for (pip_idx, pip) in cost.pips().iter().enumerate() {
        // Check if this is an X pip
        if pip.iter().any(|s| matches!(s, ManaSymbol::X)) {
            // Expand X into x_value generic pips
            for _ in 0..x_value {
                generic_pips.push(vec![ManaSymbol::Generic(1)]);
            }
        } else if pip.iter().all(|s| matches!(s, ManaSymbol::Generic(0))) {
            // Skip Generic(0) pips - they represent zero cost
            continue;
        } else if pip.len() == 1 {
            // Single-symbol pip - check if it's Generic(N) that needs expansion
            if let ManaSymbol::Generic(n) = pip[0] {
                if n > 1 {
                    // Expand Generic(N) into N individual Generic(1) pips
                    for _ in 0..n {
                        generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    }
                    continue;
                } else if n == 1 {
                    generic_pips.push(pip.clone());
                    continue;
                }
            }
            // Colored pip
            colored_pips.push(pip.clone());
        } else {
            // Multi-symbol pip (e.g., hybrid like {B/P} or {W/U})
            // Check if a choice was made during announcement stage
            if let Some((_, chosen_symbol)) = hybrid_choices.iter().find(|(idx, _)| *idx == pip_idx)
            {
                // Use the chosen symbol instead of the full alternatives
                colored_pips.push(vec![*chosen_symbol]);
            } else {
                // No choice made, keep all alternatives (shouldn't happen if announcement worked)
                colored_pips.push(pip.clone());
            }
        }
    }

    // Return colored pips first (more constrained), then generic pips (more flexible)
    colored_pips.extend(generic_pips);
    colored_pips
}

fn preferred_auto_pip_choice(
    state: &PriorityLoopState,
    options: &[ManaPipPaymentOption],
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }

    if state.auto_choose_single_pip_payment && options.len() == 1 {
        return Some(0);
    }

    if options
        .iter()
        .all(|opt| matches!(opt.action, ManaPipPaymentAction::PayViaAlternative { .. }))
    {
        return Some(0);
    }

    None
}

/// Build payment options for a single mana pip.
fn build_pip_payment_options(
    game: &GameState,
    player: PlayerId,
    pip: &[crate::mana::ManaSymbol],
    allow_any_color: bool,
    source_for_pip_alternatives: Option<ObjectId>,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPipPaymentOption> {
    use crate::mana::ManaSymbol;

    let mut options = Vec::new();
    let mut index = 0;
    let mut added_any_color_options = false;

    // Get the player's mana pool
    let pool = game.player(player).map(|p| &p.mana_pool);

    // For each alternative in the pip, check what can pay it
    for symbol in pip {
        match symbol {
            ManaSymbol::White => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.white > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {W} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::White),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Blue => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.blue > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {U} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Black => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.black > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {B} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Black),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Red => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.red > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {R} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Red),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Green => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.green > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {G} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Green),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Colorless => {
                if pool.map(|p| p.colorless > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {C} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Generic(_) => {
                // Generic can be paid with any mana in the pool
                if let Some(p) = pool {
                    add_any_color_pool_options(&mut options, &mut index, p);
                }
            }
            ManaSymbol::Life(amount) => {
                // Can always pay life (if player has enough)
                let has_life = game
                    .player(player)
                    .map(|p| p.life > *amount as i32)
                    .unwrap_or(false);
                if has_life {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: format!("Pay {} life", amount),
                        action: ManaPipPaymentAction::PayLife(*amount as u32),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Snow => {
                // Snow mana - for now treat like generic
                if let Some(p) = pool
                    && p.total() > 0
                {
                    // Just offer any available mana
                    if p.colorless > 0 {
                        options.push(ManaPipPaymentOption {
                            index,
                            description: "Use {C} from mana pool".to_string(),
                            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
                        });
                        index += 1;
                    }
                }
            }
            ManaSymbol::X => {
                // X should have been expanded already
            }
        }
    }

    add_pip_alternative_payment_options(
        game,
        player,
        pip,
        source_for_pip_alternatives,
        &mut options,
        &mut index,
    );

    // Check if this is a Phyrexian pip (has a Life alternative)
    let is_phyrexian = pip.iter().any(|s| matches!(s, ManaSymbol::Life(_)));

    // Check if we have any "use from pool" options (not just Life options)
    let has_pool_options = options
        .iter()
        .any(|opt| matches!(opt.action, ManaPipPaymentAction::UseFromPool(_)));

    // Add mana abilities if:
    // - We don't have pool options, OR
    // - This is a Phyrexian pip (always give choice between mana and life)
    if !has_pool_options || is_phyrexian {
        let mana_abilities = get_available_mana_abilities(game, player, decision_maker);
        for (perm_id, ability_index, description) in mana_abilities {
            // Check if this ability produces mana that can pay this pip
            if mana_ability_can_pay_pip(game, perm_id, ability_index, pip, allow_any_color) {
                options.push(ManaPipPaymentOption {
                    index,
                    description: format!(
                        "Tap {}: {}",
                        describe_permanent(game, perm_id),
                        description
                    ),
                    action: ManaPipPaymentAction::ActivateManaAbility {
                        source_id: perm_id,
                        ability_index,
                    },
                });
                index += 1;
            }
        }
    }

    options
}

fn add_pip_alternative_payment_options(
    game: &GameState,
    player: PlayerId,
    pip: &[crate::mana::ManaSymbol],
    source_for_pip_alternatives: Option<ObjectId>,
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
) {
    let Some(source) = source_for_pip_alternatives else {
        return;
    };
    let Some(spell) = game.object(source) else {
        return;
    };

    if crate::decision::has_convoke(spell) {
        for (creature_id, colors) in crate::decision::get_convoke_creatures(game, player) {
            if convoke_can_pay_pip(colors, pip) {
                options.push(ManaPipPaymentOption {
                    index: *index,
                    description: format!(
                        "Tap {} to pay this pip (Convoke)",
                        describe_permanent(game, creature_id)
                    ),
                    action: ManaPipPaymentAction::PayViaAlternative {
                        permanent_id: creature_id,
                        effect: AlternativePaymentEffect::Convoke,
                    },
                });
                *index += 1;
            }
        }
    }

    if crate::decision::has_improvise(spell) && improvise_can_pay_pip(pip) {
        for artifact_id in crate::decision::get_improvise_artifacts(game, player) {
            options.push(ManaPipPaymentOption {
                index: *index,
                description: format!(
                    "Tap {} to pay this pip (Improvise)",
                    describe_permanent(game, artifact_id)
                ),
                action: ManaPipPaymentAction::PayViaAlternative {
                    permanent_id: artifact_id,
                    effect: AlternativePaymentEffect::Improvise,
                },
            });
            *index += 1;
        }
    }
}

fn convoke_can_pay_pip(colors: crate::color::ColorSet, pip: &[crate::mana::ManaSymbol]) -> bool {
    pip.iter().any(|symbol| match symbol {
        crate::mana::ManaSymbol::Generic(_) => true,
        crate::mana::ManaSymbol::White => colors.contains(crate::color::Color::White),
        crate::mana::ManaSymbol::Blue => colors.contains(crate::color::Color::Blue),
        crate::mana::ManaSymbol::Black => colors.contains(crate::color::Color::Black),
        crate::mana::ManaSymbol::Red => colors.contains(crate::color::Color::Red),
        crate::mana::ManaSymbol::Green => colors.contains(crate::color::Color::Green),
        crate::mana::ManaSymbol::Colorless
        | crate::mana::ManaSymbol::Life(_)
        | crate::mana::ManaSymbol::Snow
        | crate::mana::ManaSymbol::X => false,
    })
}

fn improvise_can_pay_pip(pip: &[crate::mana::ManaSymbol]) -> bool {
    pip.iter()
        .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Generic(_)))
}

fn add_any_color_pool_options(
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
    pool: &crate::player::ManaPool,
) {
    use crate::mana::ManaSymbol;

    if pool.white > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {W} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::White),
        });
        *index += 1;
    }
    if pool.blue > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {U} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
        });
        *index += 1;
    }
    if pool.black > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {B} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Black),
        });
        *index += 1;
    }
    if pool.red > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {R} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Red),
        });
        *index += 1;
    }
    if pool.green > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {G} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Green),
        });
        *index += 1;
    }
    if pool.colorless > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {C} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
        });
        *index += 1;
    }
}

/// Check if a mana ability can produce mana that pays the given pip.
fn mana_ability_can_pay_pip(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    pip: &[crate::mana::ManaSymbol],
    allow_any_color: bool,
) -> bool {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    let Some(obj) = game.object(perm_id) else {
        return false;
    };

    let Some(ability) = obj.abilities.get(ability_index) else {
        return false;
    };

    let AbilityKind::Mana(mana_ability) = &ability.kind else {
        return false;
    };

    // Check what mana this ability produces (field is called `mana`)
    let pip_has_colored = pip.iter().any(|s| {
        matches!(
            s,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
        )
    });

    for produced in &mana_ability.mana {
        for pip_symbol in pip {
            match (produced, pip_symbol) {
                // Any mana can pay generic
                (_, ManaSymbol::Generic(_)) => return true,
                // Exact color matches
                (ManaSymbol::White, ManaSymbol::White) => return true,
                (ManaSymbol::Blue, ManaSymbol::Blue) => return true,
                (ManaSymbol::Black, ManaSymbol::Black) => return true,
                (ManaSymbol::Red, ManaSymbol::Red) => return true,
                (ManaSymbol::Green, ManaSymbol::Green) => return true,
                (ManaSymbol::Colorless, ManaSymbol::Colorless) => return true,
                // Any-color spending: any produced mana can pay colored pips
                _ if allow_any_color && pip_has_colored => return true,
                _ => {}
            }
        }
    }

    false
}

#[cfg(feature = "net")]
fn record_pip_payment_action(trace: &mut Vec<CostStep>, action: &ManaPipPaymentAction) {
    match action {
        ManaPipPaymentAction::UseFromPool(symbol) => {
            trace.push(CostStep::Mana(ManaSymbolSpec::from(*symbol)));
        }
        ManaPipPaymentAction::PayLife(amount) => {
            let capped = (*amount).min(u8::MAX as u32) as u8;
            trace.push(CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: capped,
            }));
        }
        ManaPipPaymentAction::ActivateManaAbility {
            source_id,
            ability_index,
        } => {
            trace.push(CostStep::Payment(CostPayment::ActivateManaAbility {
                source: GameObjectId(source_id.0),
                ability_index: (*ability_index).min(u32::MAX as usize) as u32,
            }));
        }
        ManaPipPaymentAction::PayViaAlternative { permanent_id, .. } => {
            trace.push(CostStep::Payment(CostPayment::Tap {
                objects: vec![GameObjectId(permanent_id.0)],
            }));
        }
    }
}

#[cfg(not(feature = "net"))]
fn record_pip_payment_action(_trace: &mut Vec<CostStep>, _action: &ManaPipPaymentAction) {}

#[cfg(feature = "net")]
fn record_immediate_cost_payment(
    trace: &mut Vec<CostStep>,
    cost: &crate::costs::Cost,
    source: ObjectId,
) {
    let source_id = GameObjectId(source.0);

    if cost.requires_tap() {
        trace.push(CostStep::Payment(CostPayment::Tap {
            objects: vec![source_id],
        }));
        return;
    }

    if cost.requires_untap() {
        trace.push(CostStep::Payment(CostPayment::Untap {
            objects: vec![source_id],
        }));
        return;
    }

    if cost.is_life_cost() {
        if let Some(amount) = cost.life_amount() {
            trace.push(CostStep::Payment(CostPayment::Life { amount }));
            return;
        }
    }

    if cost.is_sacrifice_self() {
        trace.push(CostStep::Payment(CostPayment::Sacrifice {
            objects: vec![source_id],
        }));
        return;
    }

    // Fallback: preserve order with an opaque payment tag.
    trace.push(CostStep::Payment(CostPayment::Other {
        tag: 0,
        data: cost.display().into_bytes(),
    }));
}

#[cfg(not(feature = "net"))]
fn record_immediate_cost_payment(
    _trace: &mut Vec<CostStep>,
    _cost: &crate::costs::Cost,
    _source: ObjectId,
) {
}

#[cfg(feature = "net")]
fn record_cast_mana_ability_payment(
    pending: &mut PendingCast,
    source: ObjectId,
    ability_index: usize,
) {
    pending
        .payment_trace
        .push(CostStep::Payment(CostPayment::ActivateManaAbility {
            source: GameObjectId(source.0),
            ability_index: ability_index.min(u32::MAX as usize) as u32,
        }));
}

#[cfg(not(feature = "net"))]
fn record_cast_mana_ability_payment(
    _pending: &mut PendingCast,
    _source: ObjectId,
    _ability_index: usize,
) {
}

#[cfg(feature = "net")]
fn record_activation_mana_ability_payment(
    pending: &mut PendingActivation,
    source: ObjectId,
    ability_index: usize,
) {
    pending
        .payment_trace
        .push(CostStep::Payment(CostPayment::ActivateManaAbility {
            source: GameObjectId(source.0),
            ability_index: ability_index.min(u32::MAX as usize) as u32,
        }));
}

#[cfg(not(feature = "net"))]
fn record_activation_mana_ability_payment(
    _pending: &mut PendingActivation,
    _source: ObjectId,
    _ability_index: usize,
) {
}

/// Execute a pip payment action.
/// Execute a pip payment action.
/// Returns true if the pip was actually paid (mana consumed or life paid),
/// false if we only generated mana (need to continue processing this pip).
fn execute_pip_payment_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    source: Option<ObjectId>,
    action: &ManaPipPaymentAction,
    decision_maker: &mut impl DecisionMaker,
    payment_trace: &mut Vec<CostStep>,
    mut mana_spent_to_cast: Option<&mut ManaPool>,
) -> Result<bool, GameLoopError> {
    match action {
        ManaPipPaymentAction::UseFromPool(symbol) => {
            if let Some(player_obj) = game.player_mut(player) {
                let success = player_obj.mana_pool.remove(*symbol, 1);
                if !success {
                    return Err(GameLoopError::InvalidState(format!(
                        "Not enough {:?} mana in pool",
                        symbol
                    )));
                }
            }
            if let Some(spent) = mana_spent_to_cast.as_deref_mut() {
                track_spent_mana_symbol(spent, *symbol);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
        ManaPipPaymentAction::ActivateManaAbility {
            source_id,
            ability_index,
        } => {
            // Activate the mana ability - this just generates mana, doesn't pay the pip
            crate::special_actions::perform_activate_mana_ability(
                game,
                player,
                *source_id,
                *ability_index,
                decision_maker,
            )?;
            record_pip_payment_action(payment_trace, action);
            Ok(false) // Pip not yet paid, just generated mana
        }
        ManaPipPaymentAction::PayLife(amount) => {
            if let Some(player_obj) = game.player_mut(player) {
                player_obj.life -= *amount as i32;
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
        ManaPipPaymentAction::PayViaAlternative {
            permanent_id,
            effect,
        } => {
            tap_permanent_with_trigger(game, trigger_queue, *permanent_id);
            if let Some(source_id) = source {
                let event = TriggerEvent::new(KeywordActionEvent::new(
                    keyword_action_from_alternative_effect(*effect),
                    player,
                    source_id,
                    1,
                ));
                queue_triggers_from_event(game, trigger_queue, event, true);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
    }
}

fn track_spent_mana_symbol(pool: &mut ManaPool, symbol: crate::mana::ManaSymbol) {
    use crate::mana::ManaSymbol;
    match symbol {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless => pool.add(symbol, 1),
        ManaSymbol::Generic(_) | ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X => {}
    }
}

/// Format a pip for display.
fn format_pip(pip: &[crate::mana::ManaSymbol]) -> String {
    use crate::mana::ManaSymbol;

    if pip.len() == 1 {
        // Single symbol
        match &pip[0] {
            ManaSymbol::White => "{W}".to_string(),
            ManaSymbol::Blue => "{U}".to_string(),
            ManaSymbol::Black => "{B}".to_string(),
            ManaSymbol::Red => "{R}".to_string(),
            ManaSymbol::Green => "{G}".to_string(),
            ManaSymbol::Colorless => "{C}".to_string(),
            ManaSymbol::Generic(n) => format!("{{{}}}", n),
            ManaSymbol::Snow => "{S}".to_string(),
            ManaSymbol::Life(n) => format!("{{Pay {} life}}", n),
            ManaSymbol::X => "{X}".to_string(),
        }
    } else {
        // Hybrid/Phyrexian - show alternatives
        let parts: Vec<String> = pip
            .iter()
            .map(|s| match s {
                ManaSymbol::White => "W".to_string(),
                ManaSymbol::Blue => "U".to_string(),
                ManaSymbol::Black => "B".to_string(),
                ManaSymbol::Red => "R".to_string(),
                ManaSymbol::Green => "G".to_string(),
                ManaSymbol::Colorless => "C".to_string(),
                ManaSymbol::Generic(n) => format!("{}", n),
                ManaSymbol::Snow => "S".to_string(),
                ManaSymbol::Life(n) => format!("{} life", n),
                ManaSymbol::X => "X".to_string(),
            })
            .collect();
        format!("{{{}}}", parts.join("/"))
    }
}

/// Apply a modes response to the pending cast.
///
/// This handles the player's mode selection for modal spells per MTG rule 601.2b.
fn apply_modes_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    modes: &[usize],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for modes response".to_string())
    })?;

    // Store the chosen modes
    pending.chosen_modes = Some(modes.to_vec());

    // Continue to optional costs
    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an optional costs response to the pending cast.
fn apply_optional_costs_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choices: &[(usize, u32)],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for optional costs response".to_string())
    })?;

    // Store the optional costs paid
    for &(index, times) in choices {
        pending.optional_costs_paid.pay_times(index, times);
    }

    // Continue to targeting or finalization
    continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a hybrid/Phyrexian mana choice response to a pending cast or activation.
///
/// Per MTG rule 601.2b (and 602.2b for abilities), players announce how they'll pay
/// hybrid/Phyrexian costs before choosing targets. This handler stores the choice
/// and either prompts for the next pip or continues to target selection.
fn apply_hybrid_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if this is for a pending cast (spell) or pending activation (ability)
    if let Some(mut pending) = state.pending_cast.take() {
        // Handle spell casting hybrid choice
        if pending.pending_hybrid_pips.is_empty() {
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(
                "No pending hybrid pips for hybrid choice response".to_string(),
            ));
        }

        let (pip_idx, alternatives) = pending.pending_hybrid_pips.remove(0);

        if choice >= alternatives.len() {
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(format!(
                "Invalid hybrid choice {} for pip with {} alternatives",
                choice,
                alternatives.len()
            )));
        }

        let chosen_symbol = alternatives[choice];
        pending.hybrid_choices.push((pip_idx, chosen_symbol));

        if !pending.pending_hybrid_pips.is_empty() {
            return prompt_for_next_hybrid_pip(game, state, pending);
        }

        return continue_to_targets_or_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    if let Some(mut pending) = state.pending_activation.take() {
        // Handle ability activation hybrid choice (per MTG rule 602.2b)
        if pending.pending_hybrid_pips.is_empty() {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(
                "No pending hybrid pips for hybrid choice response (activation)".to_string(),
            ));
        }

        let (pip_idx, alternatives) = pending.pending_hybrid_pips.remove(0);

        if choice >= alternatives.len() {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(format!(
                "Invalid hybrid choice {} for pip with {} alternatives (activation)",
                choice,
                alternatives.len()
            )));
        }

        let chosen_symbol = alternatives[choice];
        pending.hybrid_choices.push((pip_idx, chosen_symbol));

        // Keep stage as AnnouncingCost and let continue_activation handle the transition
        // This ensures the validation logic runs when all pips have been announced
        pending.stage = ActivationStage::AnnouncingCost;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    Err(GameLoopError::InvalidState(
        "No pending cast or activation for hybrid choice response".to_string(),
    ))
}

/// Apply a mana payment response to the pending cast.
///
/// The choice index corresponds to either:
/// - A mana ability to activate (index < num_mana_abilities)
/// - The "pay mana cost" option (last option)
fn apply_mana_payment_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::special_actions::{SpecialAction, perform};

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for mana payment response".to_string())
    })?;

    // Get the available mana abilities to determine what the choice means
    let mana_abilities = get_available_mana_abilities(game, pending.caster, decision_maker);

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability
        let (perm_id, ability_index, _) = mana_abilities[choice];

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.caster, &mut *decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.caster, true, None);

        // Record the mana ability activation in the payment trace.
        record_cast_mana_ability_payment(&mut pending, perm_id, ability_index);

        continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Player chose to pay mana cost.
        // Route to pip-by-pip payment for deterministic trace.
        continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Apply a mana payment response for a pending mana ability activation.
///
/// Mana abilities don't use the stack, so when the player can pay,
/// we immediately execute the ability.
fn apply_mana_payment_response_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::ability::AbilityKind;
    use crate::special_actions::{SpecialAction, perform};

    let pending = state.pending_mana_ability.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending mana ability for payment response".to_string())
    })?;

    // Get available mana abilities, excluding the one we're paying for
    // and filtered to only those that can help pay the cost
    let allow_any_color = game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
    let mana_abilities: Vec<_> =
        get_available_mana_abilities(game, pending.activator, decision_maker)
            .into_iter()
            .filter(|(perm_id, ability_index, _)| {
                // Exclude the ability we're paying for
                if *perm_id == pending.source && *ability_index == pending.ability_index {
                    return false;
                }

                // Check if this ability can help pay the cost
                if let Some(perm) = game.object(*perm_id)
                    && let Some(ability) = perm.abilities.get(*ability_index)
                    && let AbilityKind::Mana(mana_ability) = &ability.kind
                {
                    mana_can_help_pay_cost(
                        &mana_ability.mana,
                        &pending.mana_cost,
                        game,
                        pending.activator,
                        allow_any_color,
                    )
                } else {
                    true // If we can't determine, include it
                }
            })
            .collect();

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability to generate mana
        let (perm_id, ability_index, _) = mana_abilities[choice].clone();

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.activator, decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.activator, true, None);

        // Check if player can now pay
        let can_pay_now = game.can_pay_mana_cost(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
        );

        if can_pay_now {
            // Execute the pending mana ability
            execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)?;
            // Player retains priority after activating mana ability
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        } else {
            // Still need more mana, show options again
            let options = compute_mana_ability_payment_options(
                game,
                pending.activator,
                &pending,
                &mut *decision_maker,
            );
            let source = pending.source;
            let player = pending.activator;
            let ability_name = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());
            state.pending_mana_ability = Some(pending);

            // Convert ManaPaymentOption to SelectableOption
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();

            let ctx = crate::decisions::context::SelectOptionsContext::mana_payment(
                player,
                source,
                ability_name,
                selectable_options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
    } else {
        // Player chose to pay mana cost
        // Verify they can actually pay
        if !game.can_pay_mana_cost(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
        ) {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost - insufficient mana".to_string(),
            ));
        }

        // Execute the pending mana ability
        execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)?;
        // Player retains priority after activating mana ability
        advance_priority_with_dm(game, trigger_queue, decision_maker)
    }
}

/// Execute a pending mana ability after its mana cost has been paid.
fn execute_pending_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::costs::CostContext;
    use crate::executor::ExecutionContext;

    // Pay the mana cost
    if !game.try_pay_mana_cost(
        pending.activator,
        Some(pending.source),
        &pending.mana_cost,
        0,
    ) {
        return Err(GameLoopError::InvalidState(
            "Failed to pay mana cost".to_string(),
        ));
    }

    // Pay other costs from TotalCost (not cost_effects)
    let mut cost_ctx = CostContext::new(pending.source, pending.activator, decision_maker);
    for c in &pending.other_costs {
        crate::special_actions::pay_cost_component_with_choice(game, c, &mut cost_ctx)
            .map_err(|e| GameLoopError::InvalidState(format!("Failed to pay cost: {:?}", e)))?;
    }
    drain_pending_trigger_events(game, trigger_queue);

    // Execute the mana ability effects
    if let Some(ref effects) = pending.effects {
        let mut ctx = ExecutionContext::new(pending.source, pending.activator, decision_maker);
        let mut emitted_events = Vec::new();

        for effect in effects {
            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                emitted_events.extend(outcome.events);
            }
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    } else {
        // Add fixed mana to player's pool
        if let Some(player_obj) = game.player_mut(pending.activator) {
            for symbol in &pending.mana_to_add {
                player_obj.mana_pool.add(*symbol, 1);
            }
        }
    }

    game.record_ability_activation(pending.source, pending.ability_index);

    queue_ability_activated_event(
        game,
        trigger_queue,
        pending.source,
        pending.activator,
        true,
        None,
    );

    Ok(())
}

/// Apply a mana payment response for a pending activation.
fn apply_mana_payment_response_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::special_actions::{SpecialAction, perform};

    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for mana payment response".to_string())
    })?;

    let mana_abilities = get_available_mana_abilities(game, pending.activator, decision_maker);

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability
        let (perm_id, ability_index, _) = mana_abilities[choice];

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.activator, &mut *decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.activator, true, None);

        // Record the mana ability activation in the payment trace.
        record_activation_mana_ability_payment(&mut pending, perm_id, ability_index);

        // Stay in PayingMana stage, continue activation
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Player chose to pay mana cost
        // Verify they can actually pay
        let x_value = pending.x_value.unwrap_or(0) as u32;
        if let Some(ref cost) = pending.mana_cost_to_pay
            && !game.can_pay_mana_cost(pending.activator, Some(pending.source), cost, x_value)
        {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost - insufficient mana".to_string(),
            ));
        }

        // Pay the mana and finalize
        let mut pending = pending;
        if let Some(ref cost) = pending.mana_cost_to_pay {
            let allow_any_color =
                game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
            if let Some(player) = game.player_mut(pending.activator) {
                // Pay mana and track life for Phyrexian costs
                let (_, life_to_pay) = player.mana_pool.try_pay_tracking_life_with_any_color(
                    cost,
                    x_value,
                    allow_any_color,
                );
                // Deduct life for Phyrexian mana that couldn't be paid with mana
                if life_to_pay > 0 {
                    player.life -= life_to_pay as i32;
                }
            }
        }
        pending.stage = ActivationStage::ReadyToFinalize;
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Apply a pip payment response for a pending activation.
fn apply_pip_payment_response_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for pip payment response".to_string())
    })?;

    // Get the current pip being paid
    if pending.remaining_mana_pips.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No remaining pips to pay".to_string(),
        ));
    }

    let pip = pending.remaining_mana_pips[0].clone();

    // Rebuild the options to get the action for this choice
    let allow_any_color = game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
    let options = build_pip_payment_options(
        game,
        pending.activator,
        &pip,
        allow_any_color,
        None,
        &mut *decision_maker,
    );

    if choice >= options.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid pip payment choice: {} >= {}",
            choice,
            options.len()
        )));
    }

    let action = &options[choice].action;

    // Execute the payment action
    let pip_paid = execute_pip_payment_action(
        game,
        trigger_queue,
        pending.activator,
        Some(pending.source),
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        None,
    )?;
    queue_mana_ability_event_for_action(game, trigger_queue, action, pending.activator);
    drain_pending_trigger_events(game, trigger_queue);

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        pending.remaining_mana_pips.remove(0);
    }

    // Continue activation (will process next pip or finalize)
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a pip payment response for a pending spell cast.
fn apply_pip_payment_response_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for pip payment response".to_string())
    })?;

    // Get the current pip being paid
    if pending.remaining_mana_pips.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No remaining pips to pay".to_string(),
        ));
    }

    let pip = pending.remaining_mana_pips[0].clone();

    // Rebuild the options to get the action for this choice
    let allow_any_color = game.can_spend_mana_as_any_color(pending.caster, Some(pending.spell_id));
    let options = build_pip_payment_options(
        game,
        pending.caster,
        &pip,
        allow_any_color,
        Some(pending.spell_id),
        &mut *decision_maker,
    );

    if choice >= options.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid pip payment choice: {} >= {}",
            choice,
            options.len()
        )));
    }

    let action = &options[choice].action;

    // Execute the payment action
    let pip_paid = execute_pip_payment_action(
        game,
        trigger_queue,
        pending.caster,
        Some(pending.spell_id),
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        Some(&mut pending.mana_spent_to_cast),
    )?;
    queue_mana_ability_event_for_action(game, trigger_queue, action, pending.caster);
    drain_pending_trigger_events(game, trigger_queue);

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        record_keyword_payment_contribution(&mut pending.keyword_payment_contributions, action);
        pending.remaining_mana_pips.remove(0);
    }

    // Continue spell cast mana payment (will process next pip or finalize)
    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a sacrifice target response for a pending activation.
fn apply_sacrifice_target_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    target_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for sacrifice response".to_string())
    })?;

    // Sacrifice the chosen permanent
    if game.object(target_id).is_some() {
        let snapshot = game
            .object(target_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        let sacrificing_player = snapshot
            .as_ref()
            .map(|snap| snap.controller)
            .or(Some(pending.activator));
        game.move_object(target_id, Zone::Graveyard);
        game.queue_trigger_event(TriggerEvent::new(
            SacrificeEvent::new(target_id, Some(pending.source))
                .with_snapshot(snapshot, sacrificing_player),
        ));

        #[cfg(feature = "net")]
        {
            // Record sacrifice payment for deterministic trace
            pending
                .payment_trace
                .push(CostStep::Payment(CostPayment::Sacrifice {
                    objects: vec![GameObjectId(target_id.0)],
                }));
        }
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Remove the satisfied sacrifice cost
    if !pending.remaining_sacrifice_costs.is_empty() {
        pending.remaining_sacrifice_costs.remove(0);
    }

    // Continue activation process
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a card to exile response for a pending spell cast with alternative cost.
fn apply_card_to_exile_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    card_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for card to exile response".to_string())
    })?;

    // Add the chosen card to the exile list
    pending.cards_to_exile.push(card_id);

    // Continue with mana payment (or finalize if no mana needed)
    // We need to re-run the logic from continue_to_mana_payment
    use crate::decision::calculate_effective_mana_cost_for_payment_with_targets;

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
        let base_cost = match &pending.casting_method {
            CastingMethod::Normal => obj.mana_cost.clone(),
            CastingMethod::Alternative(idx) => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
            CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
            CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                ..
            } => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
        };
        base_cost.map(|bc| {
            calculate_effective_mana_cost_for_payment_with_targets(
                game,
                pending.caster,
                obj,
                &bc,
                pending.chosen_targets.len(),
            )
        })
    } else {
        None
    };

    pending.mana_cost_to_pay = effective_cost.clone();
    pending.stage = CastStage::PayingMana;

    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a casting method choice response for a pending spell with multiple methods.
fn apply_casting_method_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice_idx: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_method_selection.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending method selection for choice response".to_string())
    })?;

    // Get the chosen method
    let chosen_option = pending
        .available_methods
        .get(choice_idx)
        .ok_or_else(|| ResponseError::IllegalChoice("Invalid casting method choice".to_string()))?;

    let casting_method = chosen_option.method.clone();

    // Now continue with the normal spell casting flow using the chosen method
    // This is essentially a copy of the CastSpell handling logic
    let player = pending.caster;
    let spell_id = pending.spell_id;
    let from_zone = pending.from_zone;

    // Move spell to stack immediately per MTG rule 601.2a
    let stack_id = propose_spell_cast(game, spell_id, from_zone, player)?;

    // Get the spell's mana cost and effects, considering casting method
    // Note: We use stack_id now since the spell has been moved to stack
    let (mana_cost, effects) = if let Some(obj) = game.object(stack_id) {
        let cost = match &casting_method {
            CastingMethod::Normal => obj.mana_cost.clone(),
            CastingMethod::Alternative(idx) => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
            CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
            CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                ..
            } => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
        };
        (cost, obj.spell_effect.clone().unwrap_or_default())
    } else {
        (None, Vec::new())
    };

    // Check if spell has X in cost
    let has_x = mana_cost.as_ref().map(|cost| cost.has_x()).unwrap_or(false);

    if has_x {
        // Need to choose X value first - use potential mana (pool + untapped sources)
        let max_x = if let Some(ref cost) = mana_cost {
            let allow_any_color = game.can_spend_mana_as_any_color(player, Some(stack_id));
            compute_potential_mana(game, player)
                .max_x_for_cost_with_any_color(cost, allow_any_color)
        } else {
            0
        };

        // Extract target requirements for later (use stack_id since spell is on stack)
        let requirements = extract_target_requirements(game, &effects, player, Some(stack_id));

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        state.pending_cast = Some(PendingCast {
            spell_id: stack_id, // Use stack_id since spell is now on stack
            from_zone,
            caster: player,
            stage: CastStage::ChoosingX,
            x_value: None,
            chosen_targets: Vec::new(),
            remaining_requirements: requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            remaining_mana_pips: Vec::new(),
            cards_to_exile: Vec::new(),
            chosen_modes: None,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
        });

        let ctx = crate::decisions::context::NumberContext::x_value(
            player, stack_id, // Use stack_id
            max_x,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Number(ctx),
        ))
    } else {
        // No X cost, check for optional costs then targets
        let requirements = extract_target_requirements(game, &effects, player, Some(stack_id));

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        let new_pending = PendingCast {
            spell_id: stack_id, // Use stack_id since spell is now on stack
            from_zone,
            caster: player,
            stage: CastStage::ChoosingModes, // Will be updated by helper
            x_value: None,
            chosen_targets: Vec::new(),
            remaining_requirements: requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            remaining_mana_pips: Vec::new(),
            cards_to_exile: Vec::new(),
            chosen_modes: None,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
        };

        check_modes_or_continue(game, trigger_queue, state, new_pending, decision_maker)
    }
}

/// Move a spell to the stack at the start of casting (per MTG rule 601.2a).
///
/// This is called during the proposal phase, before any choices are made.
/// If casting fails later (e.g., can't pay costs), the spell should be reverted.
///
/// Returns the new ObjectId on the stack.
fn propose_spell_cast(
    game: &mut GameState,
    spell_id: ObjectId,
    _from_zone: Zone,
    _caster: PlayerId,
) -> Result<ObjectId, GameLoopError> {
    let new_id = game.move_object(spell_id, Zone::Stack).ok_or_else(|| {
        GameLoopError::InvalidState("Failed to move spell to stack during proposal".to_string())
    })?;
    Ok(new_id)
}

/// Revert a spell cast that failed during the casting process.
///
/// Per MTG rules, if casting fails at any point before completion,
/// the game state returns to before the cast was proposed.
#[allow(dead_code)]
fn revert_spell_cast(game: &mut GameState, stack_id: ObjectId, original_zone: Zone) {
    // Move spell back to original zone
    game.move_object(stack_id, original_zone);
    // Note: Mana abilities activated during casting are NOT reverted per rules
    // (they happen in a special window and their effects stay)
}

/// Result of finalizing a spell cast, containing info needed for triggers.
struct SpellCastResult {
    /// The new object ID of the spell on the stack
    new_id: ObjectId,
    /// Who cast the spell
    caster: PlayerId,
    /// Which zone the spell was cast from.
    from_zone: Zone,
}

/// Finalize a spell cast by paying remaining costs and creating the stack entry.
/// Returns the spell cast info for trigger checking.
///
/// `stack_id` is the spell already moved to stack during proposal (per 601.2a).
fn finalize_spell_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    _state: &mut PriorityLoopState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    targets: Vec<Target>,
    x_value: Option<u32>,
    casting_method: CastingMethod,
    optional_costs_paid: OptionalCostsPaid,
    chosen_modes: Option<Vec<usize>>,
    pre_chosen_exile_cards: Vec<ObjectId>,
    mut mana_spent_to_cast: ManaPool,
    keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    payment_trace: &mut Vec<CostStep>,
    mana_already_paid: bool,
    stack_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<SpellCastResult, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_with_targets;
    #[cfg(not(feature = "net"))]
    let _ = payment_trace;

    // Get the mana cost, alternative cost effects, and exile count based on casting method
    let (base_mana_cost, alternative_cost_effects, granted_escape_exile_count) =
        if let Some(obj) = game.object(spell_id) {
            match &casting_method {
                CastingMethod::Normal => (obj.mana_cost.clone(), Vec::new(), None),
                CastingMethod::Alternative(idx) => {
                    if let Some(method) = obj.alternative_casts.get(*idx) {
                        let cost_effects = method.cost_effects().to_vec();
                        if !cost_effects.is_empty() {
                            // Composed alternative method (Force of Will style) - uses cost_effects.
                            let mana = method.mana_cost().cloned();
                            (mana, cost_effects, None)
                        } else {
                            // Other alternative methods (flashback, escape, etc.) - uses mana cost only
                            let mana = method
                                .mana_cost()
                                .cloned()
                                .or_else(|| obj.mana_cost.clone());
                            (mana, Vec::new(), None)
                        }
                    } else {
                        (obj.mana_cost.clone(), Vec::new(), None)
                    }
                }
                CastingMethod::GrantedEscape { exile_count, .. } => {
                    (obj.mana_cost.clone(), Vec::new(), Some(*exile_count))
                }
                CastingMethod::GrantedFlashback => (obj.mana_cost.clone(), Vec::new(), None),
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => {
                    // Yawgmoth's Will normal cost
                    (obj.mana_cost.clone(), Vec::new(), None)
                }
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    ..
                } => {
                    // Yawgmoth's Will with alternative cost (like Force of Will)
                    if let Some(method) = obj.alternative_casts.get(*idx) {
                        let cost_effects = method.cost_effects().to_vec();
                        if !cost_effects.is_empty() {
                            let mana = method.mana_cost().cloned();
                            (mana, cost_effects, None)
                        } else {
                            let mana = method
                                .mana_cost()
                                .cloned()
                                .or_else(|| obj.mana_cost.clone());
                            (mana, Vec::new(), None)
                        }
                    } else {
                        (obj.mana_cost.clone(), Vec::new(), None)
                    }
                }
            }
        } else {
            (None, Vec::new(), None)
        };

    // Execute alternative cost effects (Force of Will style) if present
    if !alternative_cost_effects.is_empty() {
        // Build pre-chosen cards for ExileFromHand cost effects if not already provided
        let cards_for_exile = if !pre_chosen_exile_cards.is_empty() {
            pre_chosen_exile_cards.clone()
        } else {
            // Auto-select: find matching cards for any exile from hand cost effects
            let mut auto_selected = Vec::new();
            for effect in &alternative_cost_effects {
                if let Some((count, color_filter)) = effect.0.exile_from_hand_cost_info()
                    && let Some(player) = game.player(caster)
                {
                    let matching_cards: Vec<ObjectId> = player
                        .hand
                        .iter()
                        .filter(|&&card_id| {
                            if card_id == spell_id {
                                return false;
                            }
                            // Don't include cards already selected
                            if auto_selected.contains(&card_id) {
                                return false;
                            }
                            if let Some(filter) = color_filter {
                                if let Some(card) = game.object(card_id) {
                                    let card_colors = card.colors();
                                    !card_colors.intersection(filter).is_empty()
                                } else {
                                    false
                                }
                            } else {
                                true
                            }
                        })
                        .take(count as usize)
                        .copied()
                        .collect();
                    auto_selected.extend(matching_cards);
                }
            }
            auto_selected
        };

        #[cfg(feature = "net")]
        {
            // Record deterministic cost payments for alternative cost effects
            let mut exile_cursor = 0usize;
            for effect in &alternative_cost_effects {
                let mut recorded = false;

                if let Some(amount) = effect.0.pay_life_amount() {
                    payment_trace.push(CostStep::Payment(CostPayment::Life { amount }));
                    recorded = true;
                }

                if effect.0.is_tap_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Tap {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if effect.0.is_sacrifice_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Sacrifice {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if let Some((count, _)) = effect.0.exile_from_hand_cost_info() {
                    let end = (exile_cursor + count as usize).min(cards_for_exile.len());
                    let slice = &cards_for_exile[exile_cursor..end];
                    if !slice.is_empty() {
                        payment_trace.push(CostStep::Payment(CostPayment::Exile {
                            objects: slice.iter().map(|id| GameObjectId(id.0)).collect(),
                            from_zone: ZoneCode::Hand,
                        }));
                        recorded = true;
                    }
                    exile_cursor = end;
                }

                if !recorded {
                    if let Some(desc) = effect.0.cost_description() {
                        payment_trace.push(CostStep::Payment(CostPayment::Other {
                            tag: 1,
                            data: desc.into_bytes(),
                        }));
                    }
                }
            }
        }

        // Execute all cost effects with EventCause::from_cost for proper trigger handling
        let mut ctx = ExecutionContext::new(spell_id, caster, decision_maker)
            .with_cause(EventCause::from_cost(spell_id, caster));
        if let Some(x) = x_value {
            ctx = ctx.with_x(x);
        }
        // Note: cards_for_exile contains pre-selected cards for exile from hand costs.
        // The effect will auto-select the same cards when executed.
        let _ = cards_for_exile; // Silence unused variable warning

        let mut emitted_events = Vec::new();
        for effect in &alternative_cost_effects {
            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                emitted_events.extend(outcome.events);
            }
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Calculate effective cost and Delve exile count
    let (effective_cost, delve_exile_count) = if let Some(ref base_cost) = base_mana_cost {
        if let Some(obj) = game.object(spell_id) {
            let eff_cost = calculate_effective_mana_cost_with_targets(
                game,
                caster,
                obj,
                base_cost,
                targets.len(),
            );
            let delve_count = crate::decision::calculate_delve_exile_count_with_targets(
                game,
                caster,
                obj,
                base_cost,
                targets.len(),
            );
            (Some(eff_cost), delve_count)
        } else {
            (base_mana_cost.clone(), 0)
        }
    } else {
        (None, 0)
    };

    // Pay Delve cost (exile cards from graveyard)
    if delve_exile_count > 0 {
        // Collect cards to exile for Delve
        let cards_to_exile: Vec<ObjectId> = if let Some(player) = game.player(caster) {
            player
                .graveyard
                .iter()
                .filter(|&&id| id != spell_id) // Don't exile the spell being cast (shouldn't be in GY, but safety)
                .take(delve_exile_count as usize)
                .copied()
                .collect()
        } else {
            Vec::new()
        };

        #[cfg(feature = "net")]
        {
            if !cards_to_exile.is_empty() {
                payment_trace.push(CostStep::Payment(CostPayment::Exile {
                    objects: cards_to_exile.iter().map(|id| GameObjectId(id.0)).collect(),
                    from_zone: ZoneCode::Graveyard,
                }));
            }
        }

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object(card_id, Zone::Exile);
        }
    }

    // Pay the mana cost (using effective cost with reductions applied)
    // Skip if mana was already paid via pip-by-pip payment
    if !mana_already_paid && let Some(cost) = effective_cost {
        let x = x_value.unwrap_or(0);
        let before_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if !game.try_pay_mana_cost(caster, Some(spell_id), &cost, x) {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost".to_string(),
            ));
        }
        let after_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if let (Some(before), Some(after)) = (before_pool, after_pool) {
            mana_spent_to_cast.white += before.white.saturating_sub(after.white);
            mana_spent_to_cast.blue += before.blue.saturating_sub(after.blue);
            mana_spent_to_cast.black += before.black.saturating_sub(after.black);
            mana_spent_to_cast.red += before.red.saturating_sub(after.red);
            mana_spent_to_cast.green += before.green.saturating_sub(after.green);
            mana_spent_to_cast.colorless += before.colorless.saturating_sub(after.colorless);
        }
    }

    // Pay granted escape additional cost (exile cards from graveyard)
    if let Some(exile_count) = granted_escape_exile_count {
        // First, collect cards to exile (immutable borrow)
        let cards_to_exile: Vec<ObjectId> = if let Some(player) = game.player(caster) {
            player
                .graveyard
                .iter()
                .filter(|&&id| id != spell_id)
                .take(exile_count as usize)
                .copied()
                .collect()
        } else {
            Vec::new()
        };

        if cards_to_exile.len() < exile_count as usize {
            return Err(GameLoopError::InvalidState(
                "Not enough cards in graveyard to exile for escape".to_string(),
            ));
        }

        #[cfg(feature = "net")]
        {
            if !cards_to_exile.is_empty() {
                payment_trace.push(CostStep::Payment(CostPayment::Exile {
                    objects: cards_to_exile.iter().map(|id| GameObjectId(id.0)).collect(),
                    from_zone: ZoneCode::Graveyard,
                }));
            }
        }

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object(card_id, Zone::Exile);
        }
    }

    // Execute cost effects (new unified cost model)
    // Cost effects use EventCause::from_cost to enable triggers on cost-related events
    let cost_effects = game
        .object(spell_id)
        .map(|o| o.cost_effects.clone())
        .unwrap_or_default();
    if !cost_effects.is_empty() {
        #[cfg(feature = "net")]
        {
            for effect in &cost_effects {
                let mut recorded = false;

                if let Some(amount) = effect.0.pay_life_amount() {
                    payment_trace.push(CostStep::Payment(CostPayment::Life { amount }));
                    recorded = true;
                }

                if effect.0.is_tap_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Tap {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if effect.0.is_sacrifice_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Sacrifice {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if !recorded {
                    if let Some(desc) = effect.0.cost_description() {
                        payment_trace.push(CostStep::Payment(CostPayment::Other {
                            tag: 2,
                            data: desc.into_bytes(),
                        }));
                    }
                }
            }
        }

        let mut ctx = ExecutionContext::new(spell_id, caster, decision_maker)
            .with_cause(EventCause::from_cost(spell_id, caster));
        if let Some(x) = x_value {
            ctx = ctx.with_x(x);
        }
        let mut emitted_events = Vec::new();
        for effect in &cost_effects {
            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                emitted_events.extend(outcome.events);
            }
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Spell was already moved to stack during proposal (601.2a compliant).
    let new_id = stack_id;
    if let Some(spell_obj) = game.object_mut(new_id) {
        spell_obj.mana_spent_to_cast = mana_spent_to_cast;
    }

    // Create stack entry with targets, X value, casting method, optional costs, and chosen modes
    let mut entry = StackEntry::new(new_id, caster)
        .with_targets(targets.clone())
        .with_casting_method(casting_method)
        .with_optional_costs_paid(optional_costs_paid)
        .with_chosen_modes(chosen_modes)
        .with_keyword_payment_contributions(keyword_payment_contributions);
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);
    queue_becomes_targeted_events(game, trigger_queue, &targets, new_id, caster, false);

    // Track that a spell was cast this turn (per-caster)
    *game.spells_cast_this_turn.entry(caster).or_insert(0) += 1;
    game.spells_cast_this_turn_total = game.spells_cast_this_turn_total.saturating_add(1);
    game.spell_cast_order_this_turn
        .insert(new_id, game.spells_cast_this_turn_total);
    if let Some(obj) = game.object(new_id) {
        game.spells_cast_this_turn_snapshots
            .push(ObjectSnapshot::from_object(obj, game));
    }

    Ok(SpellCastResult {
        new_id,
        caster,
        from_zone,
    })
}

/// Run the priority loop using a DecisionMaker (convenience wrapper).
///
/// This drives the priority loop to completion using the provided decision maker.
/// Auto-passes priority when PassPriority is the only available action.
#[allow(clippy::never_loop)] // Loop structure is intentional for clarity
pub fn run_priority_loop_with<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    let mut state = PriorityLoopState::new(game.players_in_game());

    loop {
        // Use decision maker for triggered ability target selection
        let progress = advance_priority_with_dm(game, trigger_queue, decision_maker)?;

        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                // Handle context-based decisions in a loop
                let mut current_ctx = ctx;
                loop {
                    let auto_passed = should_auto_pass_ctx(&current_ctx);
                    let result = if auto_passed {
                        apply_priority_action_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &LegalAction::PassPriority,
                            decision_maker,
                        )
                    } else {
                        apply_decision_context_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &current_ctx,
                            decision_maker,
                        )
                    };

                    // Notify decision maker about auto-pass
                    if auto_passed && let Some(player) = get_priority_player_from_ctx(&current_ctx)
                    {
                        decision_maker.on_auto_pass(game, player);
                    }

                    // Handle errors with checkpoint rollback
                    let result = match result {
                        Ok(progress) => progress,
                        Err(e) => {
                            // Check if we have a checkpoint to restore
                            if let Some(checkpoint) = state.checkpoint.take() {
                                // Notify the decision maker about the rollback
                                decision_maker.on_action_cancelled(game, &format!("{}", e));
                                // Restore game state from checkpoint
                                *game = checkpoint;
                                // Clear any pending action state
                                state.pending_cast = None;
                                state.pending_activation = None;
                                state.pending_method_selection = None;
                                state.pending_mana_ability = None;
                                // Break from inner loop to restart with fresh priority
                                break;
                            } else {
                                // No checkpoint - propagate the error
                                return Err(e);
                            }
                        }
                    };

                    match result {
                        GameProgress::Continue => return Ok(GameProgress::Continue),
                        GameProgress::GameOver(result) => {
                            return Ok(GameProgress::GameOver(result));
                        }
                        GameProgress::NeedsDecisionCtx(next_ctx) => {
                            current_ctx = next_ctx; // Continue the context loop
                        }
                        GameProgress::StackResolved => {
                            // Stack resolved, break from inner loop to re-run advance_priority_with_dm
                            // in the outer loop with the proper decision maker for trigger targeting
                            break;
                        }
                    }
                }
            }
            GameProgress::Continue => return Ok(GameProgress::Continue),
            GameProgress::GameOver(result) => return Ok(GameProgress::GameOver(result)),
            GameProgress::StackResolved => {
                // This shouldn't happen from advance_priority_with_dm, but handle it by continuing
                continue;
            }
        }
    }
}

/// Apply a context-based decision directly using typed decision primitives.
fn apply_decision_context_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    ctx: &crate::decisions::context::DecisionContext,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    use crate::decisions::context::DecisionContext;

    match ctx {
        DecisionContext::Priority(priority_ctx) => {
            let action = decision_maker.decide_priority(game, priority_ctx);
            apply_priority_action_with_dm(game, trigger_queue, state, &action, decision_maker)
        }
        DecisionContext::Number(number_ctx) => {
            let value = decision_maker.decide_number(game, number_ctx);
            apply_x_value_response(game, trigger_queue, state, value, decision_maker)
        }
        DecisionContext::Targets(targets_ctx) => {
            let targets = decision_maker.decide_targets(game, targets_ctx);
            apply_targets_response(game, trigger_queue, state, &targets, decision_maker)
        }
        DecisionContext::Modes(modes_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = modes_ctx
                .spec
                .modes
                .iter()
                .map(|m| {
                    crate::decisions::context::SelectableOption::with_legality(
                        m.index,
                        m.description.clone(),
                        m.legal,
                    )
                })
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                modes_ctx.player,
                modes_ctx.source,
                format!("Choose mode for {}", modes_ctx.spell_name),
                options,
                modes_ctx.spec.min_modes,
                modes_ctx.spec.max_modes,
            );
            let modes = decision_maker.decide_options(game, &select_ctx);
            apply_modes_response(game, trigger_queue, state, &modes, decision_maker)
        }
        DecisionContext::HybridChoice(hybrid_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = hybrid_ctx
                .options
                .iter()
                .map(|o| crate::decisions::context::SelectableOption::new(o.index, o.label.clone()))
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                hybrid_ctx.player,
                hybrid_ctx.source,
                format!(
                    "Choose how to pay pip {} of {}",
                    hybrid_ctx.pip_number, hybrid_ctx.spell_name
                ),
                options,
                1,
                1,
            );
            let result = decision_maker.decide_options(game, &select_ctx);
            let choice = result.first().copied().ok_or_else(|| {
                GameLoopError::InvalidState("No hybrid payment choice selected".to_string())
            })?;
            apply_hybrid_choice_response(game, trigger_queue, state, choice, decision_maker)
        }
        DecisionContext::SelectObjects(objects_ctx) => {
            let result = decision_maker.decide_objects(game, objects_ctx);
            let chosen = result.first().copied().ok_or_else(|| {
                GameLoopError::InvalidState("No object selected for required choice".to_string())
            })?;

            if state.pending_activation.is_some() {
                apply_sacrifice_target_response(game, trigger_queue, state, chosen, decision_maker)
            } else if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingExileFromHand))
            {
                apply_card_to_exile_response(game, trigger_queue, state, chosen, decision_maker)
            } else {
                Err(GameLoopError::InvalidState(
                    "Unsupported SelectObjects decision in priority loop".to_string(),
                ))
            }
        }
        DecisionContext::SelectOptions(options_ctx) => {
            let result = decision_maker.decide_options(game, options_ctx);

            if game.pending_replacement_choice.is_some() {
                let choice = result.first().copied().unwrap_or(0);
                return apply_replacement_choice_response(
                    game,
                    trigger_queue,
                    choice,
                    decision_maker,
                );
            }
            if state.pending_method_selection.is_some() {
                let choice = result.first().copied().unwrap_or(0);
                return apply_casting_method_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
            {
                let choices: Vec<(usize, u32)> = result.into_iter().map(|idx| (idx, 1)).collect();
                return apply_optional_costs_response(
                    game,
                    trigger_queue,
                    state,
                    &choices,
                    decision_maker,
                );
            }
            if state.pending_mana_ability.is_some() {
                let choice = result.first().copied().unwrap_or(0);
                return apply_mana_payment_response_mana_ability(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            {
                let choice = result.first().copied().unwrap_or(0);
                return apply_pip_payment_response_activation(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::PayingMana))
            {
                let choice = result.first().copied().unwrap_or(0);
                return apply_pip_payment_response_cast(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }

            Err(GameLoopError::InvalidState(
                "Unsupported SelectOptions decision in priority loop".to_string(),
            ))
        }
        DecisionContext::Boolean(_)
        | DecisionContext::Order(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_)
        | DecisionContext::Distribute(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Counters(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => Err(GameLoopError::InvalidState(format!(
            "Unsupported decision context in priority loop: {:?}",
            std::mem::discriminant(ctx)
        ))),
    }
}

fn apply_priority_action_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    action: &LegalAction,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    match action {
        LegalAction::PassPriority => {
            let result = pass_priority(game, &mut state.tracker);

            match result {
                PriorityResult::Continue => {
                    // Next player gets priority, advance again
                    // Use decision maker for triggered ability targeting if available
                    advance_priority_with_dm(game, trigger_queue, decision_maker)
                }
                PriorityResult::StackResolves => {
                    // Resolve top of stack, passing decision maker for ETB replacements, choices, etc.
                    resolve_stack_entry_with_dm_and_triggers(game, decision_maker, trigger_queue)?;
                    // Reset priority to active player
                    reset_priority(game, &mut state.tracker);
                    // Signal that stack resolved - outer loop will call advance_priority_with_dm
                    // with the proper decision maker for trigger target selection
                    Ok(GameProgress::StackResolved)
                }
                PriorityResult::PhaseEnds => Ok(GameProgress::Continue),
            }
        }
        _ => apply_priority_response_with_dm(
            game,
            trigger_queue,
            state,
            &PriorityResponse::PriorityAction(action.clone()),
            decision_maker,
        ),
    }
}

/// Check if we should auto-pass priority for a context-based decision.
/// Returns true if this is a Priority decision with only PassPriority available.
fn should_auto_pass_ctx(ctx: &crate::decisions::context::DecisionContext) -> bool {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        pctx.legal_actions.len() == 1 && matches!(pctx.legal_actions[0], LegalAction::PassPriority)
    } else {
        false
    }
}

/// Get the player from a context-based decision, if it's a Priority decision.
fn get_priority_player_from_ctx(
    ctx: &crate::decisions::context::DecisionContext,
) -> Option<PlayerId> {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        Some(pctx.player)
    } else {
        None
    }
}

// ============================================================================
// State-Based Actions Integration
// ============================================================================

/// Check and apply all state-based actions, generating trigger events.
///
/// This runs repeatedly until no more SBAs need to be applied.
/// Note: This version auto-keeps the first legend for legend rule violations.
/// Use `check_and_apply_sbas_with` to handle legend rule interactively.
pub fn check_and_apply_sbas(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    check_and_apply_sbas_with(game, trigger_queue, &mut dm)
}

/// Check and apply all state-based actions, generating trigger events.
///
/// This runs repeatedly until no more SBAs need to be applied.
/// Legend rule violations will prompt the decision maker for which legend to keep.
pub fn check_and_apply_sbas_with(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::decisions::make_decision;
    use crate::rules::state_based::{apply_legend_rule_choice, get_legend_rule_specs};

    // Refresh continuous state (static ability effects and "can't" effect tracking)
    // before checking SBAs. This ensures the layer system is up to date.
    game.refresh_continuous_state();

    loop {
        let actions = check_state_based_actions(game);
        if actions.is_empty() {
            break;
        }

        // Handle legend rule decisions first
        let legend_specs = get_legend_rule_specs(game);
        let had_legend_decisions = !legend_specs.is_empty();
        for (player, spec) in legend_specs {
            let keep_id: ObjectId = make_decision(game, decision_maker, player, None, spec);
            apply_legend_rule_choice(game, keep_id);
        }

        // Apply the SBAs (legend rule already handled above)
        // Use the decision maker version to allow interactive replacement effect choices
        let applied = apply_state_based_actions_with(game, decision_maker);
        // SBA moves queue primitive ZoneChangeEvent via move_object; consume them now.
        drain_pending_trigger_events(game, trigger_queue);
        if !applied && !had_legend_decisions {
            break;
        }
    }

    Ok(())
}

/// Put triggered abilities from the queue onto the stack.
pub fn put_triggers_on_stack(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(game, trigger_queue, &mut dm)
}

/// Put triggered abilities from the queue onto the stack with target selection.
///
/// This handles the full flow of putting triggers on the stack:
/// 1. Group triggers by controller (APNAP order)
/// 2. For each trigger, handle target selection if needed
/// 3. Push the trigger onto the stack with targets
pub fn put_triggers_on_stack_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    // Group triggers by controller (APNAP order)
    let active_player = game.turn.active_player;
    let mut active_triggers = Vec::new();
    let mut other_triggers = Vec::new();

    for trigger in trigger_queue.take_all() {
        if trigger.controller == active_player {
            active_triggers.push(trigger);
        } else {
            other_triggers.push(trigger);
        }
    }

    // Active player's triggers go on stack first (resolve last)
    for trigger in active_triggers {
        if !can_stack_trigger_this_turn(game, &trigger) {
            continue;
        }
        if let Some(entry) =
            create_triggered_stack_entry_with_targets(game, &trigger, decision_maker)
        {
            game.record_trigger_fired(trigger.source, trigger.trigger_identity);
            game.push_to_stack(entry);
        }
    }

    // Then other players' triggers (in turn order)
    for trigger in other_triggers {
        if !can_stack_trigger_this_turn(game, &trigger) {
            continue;
        }
        if let Some(entry) =
            create_triggered_stack_entry_with_targets(game, &trigger, decision_maker)
        {
            game.record_trigger_fired(trigger.source, trigger.trigger_identity);
            game.push_to_stack(entry);
        }
    }

    Ok(())
}

fn can_stack_trigger_this_turn(game: &GameState, trigger: &TriggeredAbilityEntry) -> bool {
    let Some(ref condition) = trigger.ability.intervening_if else {
        return true;
    };

    match condition {
        crate::ability::InterveningIfCondition::FirstTimeThisTurn
        | crate::ability::InterveningIfCondition::MaxTimesEachTurn(_) => verify_intervening_if(
            game,
            condition,
            trigger.controller,
            &trigger.triggering_event,
            trigger.source,
            Some(trigger.trigger_identity),
        ),
        _ => true,
    }
}

/// Create a stack entry for a triggered ability, handling target selection.
///
/// Returns None if the trigger has mandatory targets but no legal targets exist.
fn create_triggered_stack_entry_with_targets(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<StackEntry> {
    let mut entry = triggered_to_stack_entry(game, trigger);

    // Check if this trigger has targets that need to be selected
    if trigger.ability.choices.is_empty() {
        // No targets needed
        return Some(entry);
    }

    // Select targets for each target spec
    let mut chosen_targets = Vec::new();
    for target_spec in &trigger.ability.choices {
        // Compute legal targets for this spec
        let legal_targets =
            compute_legal_targets(game, target_spec, trigger.controller, Some(trigger.source));

        if legal_targets.is_empty() {
            // No legal targets - trigger can't go on stack
            return None;
        }

        // Create a context for target selection
        let ctx = crate::decisions::context::TargetsContext::new(
            trigger.controller,
            trigger.source,
            format!("{}'s triggered ability", trigger.source_name),
            vec![crate::decisions::context::TargetRequirementContext {
                description: format!("target for {}", trigger.source_name),
                legal_targets: legal_targets.clone(),
                min_targets: 1,
                max_targets: Some(1),
            }],
        );

        // Get the choice from the decision maker
        let targets = decision_maker.decide_targets(game, &ctx);

        if let Some(first_target) = targets.first() {
            chosen_targets.push(*first_target);
        } else {
            // No target chosen - use the first legal target as default
            if let Some(first_legal) = legal_targets.first() {
                chosen_targets.push(*first_legal);
            } else {
                return None;
            }
        }
    }

    // Add the chosen targets to the stack entry
    entry.targets = chosen_targets;

    Some(entry)
}

/// Convert a triggered ability entry to a stack entry.
fn triggered_to_stack_entry(game: &GameState, trigger: &TriggeredAbilityEntry) -> StackEntry {
    use crate::events::EventKind;
    use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
    use crate::events::zones::ZoneChangeEvent;
    use crate::triggers::AttackEventTarget;

    // Capture source LKI at trigger-to-stack time. If the source no longer exists,
    // fall back to snapshot data from the triggering event (e.g. dies triggers).
    let source_snapshot = game
        .object(trigger.source)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .or_else(|| {
            trigger
                .triggering_event
                .downcast::<ZoneChangeEvent>()
                .and_then(|zc| zc.snapshot.clone())
                .filter(|snapshot| snapshot.object_id == trigger.source)
        })
        .or_else(|| {
            game.find_object_by_stable_id(trigger.source_stable_id)
                .and_then(|id| game.object(id))
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        });

    // Create an ability stack entry with the effects from the triggered ability
    let mut entry = StackEntry::ability(
        trigger.source,
        trigger.controller,
        trigger.ability.effects.clone(),
    )
    .with_source_info(trigger.source_stable_id, trigger.source_name.clone())
    .with_triggering_event(trigger.triggering_event.clone());
    if let Some(snapshot) = source_snapshot {
        entry = entry.with_source_snapshot(snapshot);
    }
    // Propagate keyword payment contributions from the source permanent's cast,
    // so triggered abilities can reference "each creature that convoked it", etc.
    if let Some(obj) = game.object(trigger.source)
        && !obj.keyword_payment_contributions_to_cast.is_empty()
    {
        entry.keyword_payment_contributions = obj.keyword_payment_contributions_to_cast.clone();
    }

    // Copy intervening-if condition if present (must be rechecked at resolution time)
    if let Some(ref condition) = trigger.ability.intervening_if {
        entry = entry.with_intervening_if(condition.clone());
    }

    // Extract defending player from combat triggers
    if trigger.triggering_event.kind() == EventKind::CreatureAttacked
        && let Some(attacked) = trigger.triggering_event.downcast::<CreatureAttackedEvent>()
    {
        match attacked.target {
            AttackEventTarget::Player(player_id) => {
                entry = entry.with_defending_player(player_id);
            }
            AttackEventTarget::Planeswalker(planeswalker_id) => {
                if let Some(planeswalker) = game.object(planeswalker_id) {
                    entry = entry.with_defending_player(planeswalker.controller);
                }
            }
        }
    }
    if trigger.triggering_event.kind() == EventKind::CreatureBecameBlocked
        && let Some(blocked) = trigger
            .triggering_event
            .downcast::<CreatureBecameBlockedEvent>()
        && let Some(target) = blocked.attack_target
    {
        match target {
            AttackEventTarget::Player(player_id) => {
                entry = entry.with_defending_player(player_id);
            }
            AttackEventTarget::Planeswalker(planeswalker_id) => {
                if let Some(planeswalker) = game.object(planeswalker_id) {
                    entry = entry.with_defending_player(planeswalker.controller);
                }
            }
        }
    }

    // Check if this is a saga's final chapter ability.
    // Use trigger metadata directly instead of parsing display strings.
    if let Some(chapters) = trigger.ability.trigger.saga_chapters()
        && let Some(saga_obj) = game.object(trigger.source)
    {
        let max_chapter = saga_obj.max_saga_chapter.unwrap_or(0);
        if chapters.iter().any(|&ch| ch >= max_chapter) {
            entry = entry.with_saga_final_chapter(trigger.source);
        }
    }

    entry
}

// ============================================================================
// Combat Decision Handling
// ============================================================================

/// Get a decision context for declaring attackers.
pub fn get_declare_attackers_decision(
    game: &GameState,
    combat: &CombatState,
) -> crate::decisions::context::DecisionContext {
    let player = game.turn.active_player;
    let legal_attackers = compute_legal_attackers(game, combat);

    // Convert to AttackersContext
    let attacker_options: Vec<crate::decisions::context::AttackerOptionContext> = legal_attackers
        .into_iter()
        .map(|opt| {
            let creature_name = game
                .object(opt.creature)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Creature #{}", opt.creature.0));
            crate::decisions::context::AttackerOptionContext {
                creature: opt.creature,
                creature_name,
                valid_targets: opt.valid_targets,
                must_attack: opt.must_attack,
            }
        })
        .collect();

    crate::decisions::context::DecisionContext::Attackers(
        crate::decisions::context::AttackersContext::new(player, attacker_options),
    )
}

/// Apply attacker declarations to the combat state.
pub fn apply_attacker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[AttackerDeclaration],
) -> Result<(), GameLoopError> {
    use crate::combat_state::AttackerInfo;
    use crate::triggers::AttackEventTarget;
    use std::collections::HashSet;

    // Validate that all creatures with "must attack if able" are declared
    let legal_attackers = compute_legal_attackers(game, combat);
    let declared_creatures: HashSet<ObjectId> = declarations.iter().map(|d| d.creature).collect();

    for attacker in &legal_attackers {
        if attacker.must_attack && !declared_creatures.contains(&attacker.creature) {
            return Err(CombatError::MustAttackNotDeclared(attacker.creature).into());
        }
    }

    // Clear any existing attackers
    combat.attackers.clear();
    if !declarations.is_empty() {
        game.players_attacked_this_turn
            .insert(game.turn.active_player);
    }

    for decl in declarations {
        let Some(legal_option) = legal_attackers
            .iter()
            .find(|option| option.creature == decl.creature)
        else {
            return Err(
                ResponseError::InvalidAttackers("Creature cannot attack".to_string()).into(),
            );
        };
        if !legal_option.valid_targets.contains(&decl.target) {
            return Err(ResponseError::InvalidAttackers(
                "Creature cannot attack the chosen target".to_string(),
            )
            .into());
        }

        // Validate the attacker
        let Some(creature) = game.object(decl.creature) else {
            return Err(ResponseError::InvalidAttackers(format!(
                "Creature {:?} not found",
                decl.creature
            ))
            .into());
        };

        if creature.controller != game.turn.active_player {
            return Err(ResponseError::InvalidAttackers(
                "Can only attack with creatures you control".to_string(),
            )
            .into());
        }

        if !creature.is_creature() {
            return Err(ResponseError::InvalidAttackers("Not a creature".to_string()).into());
        }

        // Add to combat state
        combat.attackers.push(AttackerInfo {
            creature: decl.creature,
            target: decl.target.clone(),
        });

        // Tap the creature (unless it has vigilance)
        if !crate::rules::combat::has_vigilance(creature) {
            tap_permanent_with_trigger(game, trigger_queue, decl.creature);
        }

        game.mark_creature_attacked_this_turn(decl.creature);

        // Generate attack trigger
        let event_target = match &decl.target {
            AttackTarget::Player(pid) => AttackEventTarget::Player(*pid),
            AttackTarget::Planeswalker(oid) => AttackEventTarget::Planeswalker(*oid),
        };

        let event = TriggerEvent::new(CreatureAttackedEvent::with_total_attackers(
            decl.creature,
            event_target,
            declarations.len(),
        ));
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }

    Ok(())
}

/// Get a decision context for declaring blockers.
pub fn get_declare_blockers_decision(
    game: &GameState,
    combat: &CombatState,
    defending_player: PlayerId,
) -> crate::decisions::context::DecisionContext {
    let attacker_options = compute_legal_blockers(game, combat, defending_player);

    // Convert to BlockersContext
    let blocker_options: Vec<crate::decisions::context::BlockerOptionContext> = attacker_options
        .into_iter()
        .map(|opt| {
            let attacker_name = game
                .object(opt.attacker)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Attacker #{}", opt.attacker.0));
            let valid_blockers: Vec<(ObjectId, String)> = opt
                .valid_blockers
                .into_iter()
                .map(|id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| format!("Creature #{}", id.0));
                    (id, name)
                })
                .collect();
            crate::decisions::context::BlockerOptionContext {
                attacker: opt.attacker,
                attacker_name,
                valid_blockers,
                min_blockers: opt.min_blockers,
            }
        })
        .collect();

    crate::decisions::context::DecisionContext::Blockers(
        crate::decisions::context::BlockersContext::new(defending_player, blocker_options),
    )
}

/// Apply blocker declarations to the combat state.
pub fn apply_blocker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
    defending_player: PlayerId,
) -> Result<(), GameLoopError> {
    // Clear existing blockers
    combat.blockers.clear();
    let mut seen_blockers = std::collections::HashSet::new();

    for decl in declarations {
        if !seen_blockers.insert(decl.blocker) {
            return Err(ResponseError::InvalidBlockers(
                "A creature can't block multiple attackers".to_string(),
            )
            .into());
        }

        // Validate the blocker
        let Some(blocker) = game.object(decl.blocker) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Blocker {:?} not found",
                decl.blocker
            ))
            .into());
        };

        if blocker.controller != defending_player {
            return Err(ResponseError::InvalidBlockers(
                "Can only block with creatures you control".to_string(),
            )
            .into());
        }

        if !blocker.is_creature() {
            return Err(ResponseError::InvalidBlockers("Not a creature".to_string()).into());
        }

        // Validate the attacker
        let Some(attacker) = game.object(decl.blocking) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Attacker {:?} not found",
                decl.blocking
            ))
            .into());
        };

        if !crate::rules::combat::can_block(attacker, blocker, game) {
            return Err(ResponseError::InvalidBlockers(format!(
                "{:?} can't block {:?}",
                decl.blocker, decl.blocking
            ))
            .into());
        }

        // Add to combat blockers
        combat
            .blockers
            .entry(decl.blocking)
            .or_default()
            .push(decl.blocker);

        // Generate block trigger
        let event = TriggerEvent::new(CreatureBlockedEvent::new(decl.blocker, decl.blocking));
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }

    // Generate "becomes blocked" triggers for blocked attackers
    for (attacker_id, blockers) in &combat.blockers {
        if !blockers.is_empty() {
            let attack_target =
                get_attack_target(combat, *attacker_id).map(|target| match target {
                    AttackTarget::Player(player_id) => {
                        crate::triggers::AttackEventTarget::Player(*player_id)
                    }
                    AttackTarget::Planeswalker(planeswalker_id) => {
                        crate::triggers::AttackEventTarget::Planeswalker(*planeswalker_id)
                    }
                });
            let event = TriggerEvent::new(match attack_target {
                Some(target) => CreatureBecameBlockedEvent::with_target(
                    *attacker_id,
                    blockers.len() as u32,
                    target,
                ),
                None => CreatureBecameBlockedEvent::new(*attacker_id, blockers.len() as u32),
            });
            let triggers = check_triggers(game, &event);
            for trigger in triggers {
                trigger_queue.add(trigger);
            }
        }
    }

    for (attacker_id, blockers) in &combat.blockers {
        let Some(attacker) = game.object(*attacker_id) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Attacker {:?} not found",
                attacker_id
            ))
            .into());
        };

        let min = minimum_blockers(attacker);
        if !blockers.is_empty() && blockers.len() < min {
            return Err(ResponseError::InvalidBlockers(format!(
                "{:?} needs at least {} blockers",
                attacker_id, min
            ))
            .into());
        }

        if let Some(max) = maximum_blockers(attacker, game)
            && blockers.len() > max
        {
            return Err(ResponseError::InvalidBlockers(format!(
                "{:?} can't be blocked by more than {} creature(s)",
                attacker_id, max
            ))
            .into());
        }
    }

    Ok(())
}

/// Get a decision context for ordering blockers (damage assignment order).
pub fn get_blocker_order_decision(
    game: &GameState,
    combat: &CombatState,
    attacker: ObjectId,
) -> Option<crate::decisions::context::DecisionContext> {
    // Get the blockers for this attacker
    let blockers = combat.blockers.get(&attacker)?;

    // Only need to order if there are multiple blockers
    if blockers.len() <= 1 {
        return None;
    }

    // The attacking creature's controller orders the blockers
    let attacker_obj = game.object(attacker)?;
    let attacking_player = attacker_obj.controller;

    // Convert blockers to items with names
    let items: Vec<(ObjectId, String)> = blockers
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Blocker #{}", id.0));
            (id, name)
        })
        .collect();

    let attacker_name = attacker_obj.name.clone();
    let ctx = crate::decisions::context::OrderContext::new(
        attacking_player,
        Some(attacker),
        format!("Order blockers for {}", attacker_name),
        items,
    );

    Some(crate::decisions::context::DecisionContext::Order(ctx))
}

// ============================================================================
// Full Turn Execution
// ============================================================================

/// Execute a complete turn using a DecisionMaker.
///
/// This is the full-featured version that properly handles all combat decisions.
pub fn execute_turn_with(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    // === Beginning Phase ===
    game.activate_pending_player_control(game.turn.active_player);

    // Untap step - no priority
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Untap);
    execute_untap_step(game);

    // Upkeep step
    game.turn.step = Some(Step::Upkeep);
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // Draw step
    game.turn.step = Some(Step::Draw);
    let draw_events = execute_draw_step(game);
    generate_and_queue_step_triggers(game, trigger_queue);

    // Check triggers for each drawn card (handles Miracle and other card-draw triggers)
    for draw_event in draw_events {
        let triggered = crate::triggers::check::check_triggers(game, &draw_event);
        for entry in triggered {
            trigger_queue.add(entry);
        }
    }

    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // === Precombat Main Phase ===
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);

    // Add lore counters to sagas (triggers chapter abilities)
    add_saga_lore_counters(game, trigger_queue);

    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // === Combat Phase ===
    game.turn.phase = Phase::Combat;

    // Begin combat step
    game.turn.step = Some(Step::BeginCombat);
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // Declare attackers step
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.priority_player = Some(game.turn.active_player);

    // Get attacker decision from decision maker
    let attacker_ctx = get_declare_attackers_decision(game, combat);
    let declarations: Vec<crate::decision::AttackerDeclaration> =
        if let crate::decisions::context::DecisionContext::Attackers(ctx) = &attacker_ctx {
            decision_maker
                .decide_attackers(game, ctx)
                .into_iter()
                .map(|d| crate::decision::AttackerDeclaration {
                    creature: d.creature,
                    target: d.target,
                })
                .collect()
        } else {
            Vec::new()
        };
    apply_attacker_declarations(game, combat, trigger_queue, &declarations)?;

    // Triggers from attacks, then priority
    put_triggers_on_stack(game, trigger_queue)?;
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // Declare blockers step (only if there are attackers)
    if !combat.attackers.is_empty() {
        game.turn.step = Some(Step::DeclareBlockers);

        // Determine defending player(s) - for now, assume one opponent
        let defending_player = game
            .players
            .iter()
            .find(|p| p.id != game.turn.active_player && p.is_in_game())
            .map(|p| p.id)
            .unwrap_or(game.turn.active_player);

        game.turn.priority_player = Some(defending_player);

        // Get blocker decision
        let blocker_ctx = get_declare_blockers_decision(game, combat, defending_player);
        let declarations: Vec<crate::decision::BlockerDeclaration> =
            if let crate::decisions::context::DecisionContext::Blockers(ctx) = &blocker_ctx {
                decision_maker
                    .decide_blockers(game, ctx)
                    .into_iter()
                    .map(|d| crate::decision::BlockerDeclaration {
                        blocker: d.blocker,
                        blocking: d.blocking,
                    })
                    .collect()
            } else {
                Vec::new()
            };
        apply_blocker_declarations(game, combat, trigger_queue, &declarations, defending_player)?;

        // Triggers from blocks, then priority
        put_triggers_on_stack(game, trigger_queue)?;
        run_priority_loop_with(game, trigger_queue, decision_maker)?;

        // Combat damage step
        game.turn.step = Some(Step::CombatDamage);

        // Check for first strike (use game-aware function to detect abilities from continuous effects)
        let has_first_strike = combat.attackers.iter().any(|info| {
            game.object(info.creature)
                .is_some_and(|obj| deals_first_strike_damage_with_game(obj, game))
        }) || combat.blockers.values().any(|blockers| {
            blockers.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| deals_first_strike_damage_with_game(obj, game))
            })
        });

        if has_first_strike {
            // First strike damage
            let events = execute_combat_damage_step(game, combat, true);
            generate_damage_triggers(game, &events, trigger_queue);
            check_and_apply_sbas(game, trigger_queue)?;
            run_priority_loop_with(game, trigger_queue, decision_maker)?;
        }

        // Regular damage
        let events = execute_combat_damage_step(game, combat, false);
        generate_damage_triggers(game, &events, trigger_queue);
        check_and_apply_sbas(game, trigger_queue)?;
        run_priority_loop_with(game, trigger_queue, decision_maker)?;
    }

    // End combat step
    game.turn.step = Some(Step::EndCombat);
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);
    crate::combat_state::end_combat(combat);
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // === Postcombat Main Phase ===
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // === Ending Phase ===
    game.turn.phase = Phase::Ending;

    // End step
    game.turn.step = Some(Step::End);
    game.turn.priority_player = Some(game.turn.active_player);
    generate_and_queue_step_triggers(game, trigger_queue);
    run_priority_loop_with(game, trigger_queue, decision_maker)?;

    // Cleanup step
    game.turn.step = Some(Step::Cleanup);

    // Check if discard is needed
    if let Some((player, spec)) = crate::turn::get_cleanup_discard_spec(game) {
        use crate::decisions::DecisionSpec;
        let ctx = spec.build_context(player, None, game);
        if let crate::decisions::context::DecisionContext::SelectObjects(obj_ctx) = ctx {
            let cards = decision_maker.decide_objects(game, &obj_ctx);
            crate::turn::apply_cleanup_discard(game, &cards, decision_maker);
        }
    }

    execute_cleanup_step(game);

    // If triggers fire or SBAs happen during cleanup, there's another cleanup step
    let triggers_fired = !trigger_queue.is_empty();
    let sbas_happened = !check_state_based_actions(game).is_empty();

    if triggers_fired || sbas_happened {
        check_and_apply_sbas(game, trigger_queue)?;
        put_triggers_on_stack(game, trigger_queue)?;
        if !game.stack_is_empty() {
            run_priority_loop_with(game, trigger_queue, decision_maker)?;
        }
        // Recursive cleanup - also check for discard
        if let Some((player, spec)) = crate::turn::get_cleanup_discard_spec(game) {
            use crate::decisions::DecisionSpec;
            let ctx = spec.build_context(player, None, game);
            if let crate::decisions::context::DecisionContext::SelectObjects(obj_ctx) = ctx {
                let cards = decision_maker.decide_objects(game, &obj_ctx);
                crate::turn::apply_cleanup_discard(game, &cards, decision_maker);
            }
        }
        execute_cleanup_step(game);
    }

    Ok(())
}

/// Generate step trigger events and add them to the queue.
pub fn generate_and_queue_step_triggers(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    if let Some(event) = generate_step_trigger_events(game) {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }
}

/// Generate damage trigger events from combat damage.
fn generate_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    game.clear_combat_damage_player_batch_hits();
    for event in events {
        let damage_target = match event.target {
            DamageEventTarget::Player(p) => EventDamageTarget::Player(p),
            DamageEventTarget::Object(o) => EventDamageTarget::Object(o),
        };
        let trigger_event = TriggerEvent::new(DamageEvent::new(
            event.source,
            damage_target,
            event.amount,
            true, // is_combat
        ));
        queue_triggers_from_event(game, trigger_queue, trigger_event, false);

        if let DamageEventTarget::Player(player_id) = event.target
            && event.life_lost > 0
        {
            let life_loss_event =
                TriggerEvent::new(LifeLossEvent::new(player_id, event.life_lost, true));
            queue_triggers_from_event(game, trigger_queue, life_loss_event, false);
        }

        if let DamageEventTarget::Player(player_id) = event.target
            && event.amount > 0
        {
            game.record_combat_damage_player_batch_hit(event.source, player_id);
        }
    }
    game.clear_combat_damage_player_batch_hits();
}

/// Queue combat-damage and life-loss triggers for a batch of combat damage events.
///
/// This is shared by different runtime frontends (CLI/WASM) so they can execute
/// combat damage in step actions while keeping trigger emission consistent.
#[allow(dead_code)]
pub(crate) fn queue_combat_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    generate_damage_triggers(game, events, trigger_queue);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::AttackTarget;
    use crate::decision::AutoPassDecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::events::EventKind;
    use crate::ids::CardId;
    use crate::triggers::Trigger;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_generate_damage_triggers_emits_life_loss_for_player_damage() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();

        let events = vec![CombatDamageEvent {
            source: ObjectId::from_raw(99),
            target: DamageEventTarget::Player(PlayerId::from_index(1)),
            amount: 3,
            life_lost: 3,
            result: DamageResult::default(),
        }];

        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        assert_eq!(
            game.trigger_event_kind_count_this_turn(EventKind::Damage),
            1
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(EventKind::LifeLoss),
            1
        );
        assert_eq!(
            game.damage_to_players_this_turn
                .get(&PlayerId::from_index(1)),
            Some(&3)
        );
    }

    #[test]
    fn test_drain_pending_events_checks_delayed_zone_change_triggers() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
        let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);

        game.delayed_triggers.push(crate::triggers::DelayedTrigger {
            trigger: Trigger::this_leaves_battlefield(),
            effects: vec![Effect::move_to_zone(
                ChooseSpec::SpecificObject(twin_id),
                Zone::Exile,
                true,
            )],
            one_shot: true,
            not_before_turn: None,
            expires_at_turn: None,
            target_objects: vec![stangg_id],
            ability_source: None,
            controller: alice,
        });

        let moved = game.move_object(stangg_id, Zone::Graveyard);
        assert!(moved.is_some(), "expected Stangg to move to graveyard");
        assert!(
            !game.pending_trigger_events.is_empty(),
            "moving Stangg off battlefield should queue a zone-change trigger event"
        );

        drain_pending_trigger_events(&mut game, &mut trigger_queue);

        assert!(
            !trigger_queue.entries.is_empty(),
            "pending zone-change events should check delayed triggers"
        );
        assert_eq!(
            trigger_queue.entries[0].source, stangg_id,
            "delayed trigger source should be the leaving permanent"
        );

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("delayed trigger should be put on stack");
        assert_eq!(
            game.stack.len(),
            1,
            "expected delayed trigger ability on stack"
        );

        resolve_stack_entry(&mut game).expect("delayed trigger should resolve");

        assert!(
            !game.battlefield.contains(&twin_id),
            "Stangg Twin should no longer be on battlefield after delayed exile resolves"
        );
    }

    #[test]
    fn test_pending_zone_change_still_drives_non_delayed_triggered_abilities() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
        let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);

        if let Some(stangg) = game.object_mut(stangg_id) {
            let filter = ObjectFilter::default().token().named("Stangg Twin");
            stangg.abilities.push(Ability::triggered(
                Trigger::leaves_battlefield(filter),
                vec![Effect::sacrifice_source()],
            ));
        } else {
            panic!("expected Stangg object to exist");
        }

        let moved = game.move_object(twin_id, Zone::Graveyard);
        assert!(moved.is_some(), "expected Stangg Twin to move to graveyard");

        drain_pending_trigger_events(&mut game, &mut trigger_queue);
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("triggered ability should be put on stack");
        assert_eq!(
            game.stack.len(),
            1,
            "expected sacrifice trigger on stack after Stangg Twin left"
        );

        resolve_stack_entry(&mut game).expect("sacrifice trigger should resolve");

        assert!(
            !game.battlefield.contains(&stangg_id),
            "Stangg should be sacrificed when Stangg Twin leaves the battlefield"
        );
    }

    #[test]
    fn test_stangg_linked_twin_sacrifice_survives_legend_rule_for_other_twin() {
        use crate::ability::AbilityKind;
        use crate::cards::CardDefinitionBuilder;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::ids::CardId;

        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let oracle = "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.";
        let def = CardDefinitionBuilder::new(CardId::new(), "Stangg")
            .parse_text(oracle)
            .expect("parse stangg text");
        let etb = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if format!("{:?}", triggered.effects).contains("CreateTokenEffect") =>
                {
                    Some(triggered.clone())
                }
                _ => None,
            })
            .expect("expected stangg ETB trigger");

        let stangg_a = create_creature(&mut game, "Stangg", alice, 3, 4);
        let stangg_b = create_creature(&mut game, "Stangg", alice, 3, 4);

        for source in [stangg_a, stangg_b] {
            let mut dm = crate::decision::AutoPassDecisionMaker;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            for effect in &etb.effects {
                execute_effect(&mut game, effect, &mut ctx)
                    .expect("stangg ETB effect should resolve");
            }
        }

        let twins_before_sba = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
            .count();
        assert_eq!(
            twins_before_sba, 2,
            "expected two Stangg Twin tokens before legend rule applies"
        );

        check_and_apply_sbas(&mut game, &mut trigger_queue).expect("apply SBAs");

        let twins_after_sba = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
            .count();
        assert_eq!(twins_after_sba, 1, "legend rule should keep only one Twin");

        put_triggers_on_stack(&mut game, &mut trigger_queue).expect("queue triggered abilities");
        while !game.stack_is_empty() {
            resolve_stack_entry(&mut game).expect("resolve trigger");
        }

        let stangg_after_resolution = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg"))
            .count();
        assert_eq!(
            stangg_after_resolution, 1,
            "only the Stangg linked to the Twin that left should be sacrificed"
        );
    }

    #[test]
    fn test_turn_face_up_action_puts_turned_face_up_trigger_on_stack() {
        use crate::decision::{LegalAction, SelectFirstDecisionMaker};
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::triggers::Trigger;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let card = CardBuilder::new(CardId::from_raw(42), "Face-Up Trigger Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::morph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
            obj.abilities.push(Ability::triggered(
                Trigger::this_is_turned_face_up(),
                vec![Effect::draw(1)],
            ));
        }
        game.set_face_down(creature_id);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp { creature_id });
        let result = apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &response,
            &mut dm,
        );
        assert!(result.is_ok(), "turn-face-up action should succeed");

        let top = game
            .stack
            .last()
            .expect("triggered ability should be on stack");
        assert!(
            top.is_ability,
            "turned-face-up trigger should use ability stack entry"
        );
        assert_eq!(top.object_id, creature_id);
        assert!(
            top.triggering_event
                .as_ref()
                .is_some_and(|event| event.kind() == EventKind::TurnedFaceUp),
            "trigger stack entry should carry TurnedFaceUp trigger event"
        );
    }

    // === Target Extraction Tests ===

    #[cfg(feature = "net")]
    #[test]
    fn test_pip_payment_trace_order() {
        use crate::mana::ManaSymbol;

        let mut trace = Vec::new();
        let actions = vec![
            ManaPipPaymentAction::ActivateManaAbility {
                source_id: ObjectId::from_raw(5),
                ability_index: 1,
            },
            ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
            ManaPipPaymentAction::PayViaAlternative {
                permanent_id: ObjectId::from_raw(6),
                effect: crate::decision::AlternativePaymentEffect::Convoke,
            },
            ManaPipPaymentAction::PayLife(2),
        ];

        for action in &actions {
            record_pip_payment_action(&mut trace, action);
        }

        assert_eq!(trace.len(), 4);
        assert!(matches!(
            trace[0],
            CostStep::Payment(CostPayment::ActivateManaAbility { .. })
        ));
        assert!(matches!(
            trace[1],
            CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Blue,
                ..
            })
        ));
        assert!(matches!(
            trace[2],
            CostStep::Payment(CostPayment::Tap { .. })
        ));
        assert!(matches!(
            trace[3],
            CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: 2,
            })
        ));
    }

    #[test]
    fn test_extract_target_spec_single_target() {
        // Destroy effect has single target
        let effect = Effect::destroy(ChooseSpec::creature());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        assert_eq!(extracted.min_targets, 1);
        assert_eq!(extracted.max_targets, Some(1));
    }

    #[test]
    fn test_extract_target_spec_any_number() {
        // Exile with any_number count (using exile_any_number helper)
        let effect = Effect::exile_any_number(ChooseSpec::spell());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        // ChoiceCount::any_number() returns min: 0, max: None
        assert_eq!(extracted.min_targets, 0, "any_number has min 0");
        assert_eq!(extracted.max_targets, None, "any_number has no max");
    }

    #[test]
    fn test_extract_target_spec_no_count() {
        // Exile with no count defaults to single target
        let effect = Effect::exile(ChooseSpec::creature());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        assert_eq!(extracted.min_targets, 1, "should default to min 1");
        assert_eq!(extracted.max_targets, Some(1), "should default to max 1");
    }

    #[test]
    fn test_spell_has_legal_targets_any_number_with_no_targets() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Exile any number of target spells
        let effects = vec![Effect::exile_any_number(ChooseSpec::spell())];

        // No spells on stack - but "any number" (min_targets == 0) means 0 targets is valid
        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        // "Any number" effects can be cast with 0 targets
        assert!(has_targets, "any_number effects can be cast with 0 targets");
    }

    #[test]
    fn test_spell_has_legal_targets_single_target_needs_target() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Single target exile spell - needs at least one target
        let effects = vec![Effect::exile(ChooseSpec::spell())];

        // No spells on stack
        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        assert!(
            !has_targets,
            "Single-target spell needs at least one legal target"
        );
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn undying_effects() -> Vec<Effect> {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose =
            Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );

        vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ]
    }

    fn persist_effects() -> Vec<Effect> {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose =
            Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );

        vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ]
    }

    // === Stack Resolution Tests ===

    #[test]
    fn test_resolve_empty_stack() {
        let mut game = setup_game();
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_stack_entry_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a simple instant
        let card = CardBuilder::new(CardId::from_raw(1), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);

        // Put on stack
        let entry = StackEntry::new(spell_id, alice);
        game.push_to_stack(entry);

        // Resolve
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_ok());

        // Stack should be empty
        assert!(game.stack_is_empty());

        // Spell should be in graveyard
        let player = game.player(alice).unwrap();
        assert_eq!(player.graveyard.len(), 1);
    }

    // === Combat Damage Tests ===

    #[test]
    fn test_unblocked_attacker_deals_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);

        // Set up combat with attacker attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 3);

        // Bob should have taken 3 damage
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_unblocked_attacker_uses_calculated_power_from_conditional_anthem() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Tek Test", alice, 2, 2);

        if let Some(attacker) = game.object_mut(attacker_id) {
            let swamp_condition = crate::static_abilities::StaticCondition::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(
                    crate::filter::ObjectFilter::land()
                        .with_subtype(crate::types::Subtype::Swamp)
                        .you_control(),
                ),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some("you control a Swamp".to_string()),
            };
            let anthem =
                crate::static_abilities::Anthem::for_source(2, 0).with_condition(swamp_condition);
            attacker.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::new(anthem),
            ));
        }

        let swamp = CardBuilder::new(CardId::new(), "Swamp")
            .card_types(vec![CardType::Land])
            .subtypes(vec![crate::types::Subtype::Swamp])
            .build();
        game.create_object_from_card(&swamp, alice, Zone::Battlefield);

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 4);
        assert_eq!(game.player(bob).unwrap().life, 16);
    }

    #[test]
    fn test_blocked_attacker_deals_damage_to_blocker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);
        let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, vec![blocker_id]);

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Should have events for attacker->blocker and blocker->attacker
        assert!(events.len() >= 2);

        // Blocker should have 2 damage (lethal - without trample, attacker only assigns lethal)
        assert_eq!(game.damage_on(blocker_id), 2);

        // Attacker should have 2 damage
        assert_eq!(game.damage_on(attacker_id), 2);

        // Bob should not have taken damage (attacker was blocked)
        assert_eq!(game.player(bob).unwrap().life, 20);
    }

    #[test]
    fn test_first_strike_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "First Striker", alice, 2, 2);

        // Add first strike
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::first_strike(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // First strike damage step - should deal damage
        let events = execute_combat_damage_step(&mut game, &combat, true);
        assert_eq!(events.len(), 1);
        assert_eq!(game.player(bob).unwrap().life, 18);

        // Regular damage step - first strike creature shouldn't deal damage again
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 0);
        assert_eq!(game.player(bob).unwrap().life, 18);
    }

    #[test]
    fn test_lifelink_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Lifelinker", alice, 3, 3);

        // Add lifelink
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::lifelink(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let _events = execute_combat_damage_step(&mut game, &combat, false);

        // Bob took 3 damage
        assert_eq!(game.player(bob).unwrap().life, 17);

        // Alice gained 3 life
        assert_eq!(game.player(alice).unwrap().life, 23);
    }

    #[test]
    fn test_trample_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Trampler", alice, 5, 5);
        let blocker_id = create_creature(&mut game, "Small Blocker", bob, 2, 2);

        // Add trample
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::trample(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, vec![blocker_id]);

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Should have events: attacker->blocker, attacker->player (trample), blocker->attacker
        assert!(events.len() >= 3);

        // Blocker should have 2 damage (lethal)
        assert_eq!(game.damage_on(blocker_id), 2);

        // Attacker should have 2 damage (from blocker)
        assert_eq!(game.damage_on(attacker_id), 2);

        // Bob should have taken 3 trample damage (5 power - 2 toughness = 3 excess)
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    // === State-Based Actions Tests ===

    #[test]
    fn test_sba_creature_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Doomed", alice, 2, 2);

        // Deal lethal damage
        game.mark_damage(creature_id, 2);

        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Creature should be in graveyard
        assert_eq!(game.battlefield.len(), 0);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_sba_player_loses() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set life to 0
        game.player_mut(alice).unwrap().life = 0;

        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Alice should have lost
        assert!(game.player(alice).unwrap().has_lost);
    }

    // === Priority Loop Tests ===

    #[test]
    fn test_priority_loop_empty_stack() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();

        // With empty stack and all passing, phase should end
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm).unwrap();
        assert!(matches!(result, GameProgress::Continue));
    }

    // === Triggered Ability Tests ===

    #[test]
    fn test_etb_trigger_fires() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create creature with ETB trigger
        let creature_id = create_creature(&mut game, "ETB Creature", alice, 2, 2);
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::draw(1)],
            ));
        }

        // Simulate ETB event
        let event = TriggerEvent::new(crate::events::zones::ZoneChangeEvent::new(
            creature_id,
            Zone::Stack,
            Zone::Battlefield,
            None,
        ));

        let mut trigger_queue = TriggerQueue::new();
        let triggers = check_triggers(&game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }

        assert!(!trigger_queue.is_empty());
        assert_eq!(trigger_queue.entries.len(), 1);
    }

    #[test]
    fn test_dies_trigger_from_sba() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist-like creature
        let blood_artist_id = create_creature(&mut game, "Blood Artist", alice, 0, 1);
        if let Some(obj) = game.object_mut(blood_artist_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::dies(crate::target::ObjectFilter::creature()),
                vec![Effect::gain_life(1)],
            ));
        }

        // Create victim creature with lethal damage
        let victim_id = create_creature(&mut game, "Victim", alice, 1, 1);
        game.mark_damage(victim_id, 1);

        // Apply SBAs - should trigger Blood Artist
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Blood Artist should have triggered
        assert!(!trigger_queue.is_empty());
    }

    // === Integration Tests ===

    #[test]
    fn test_combat_damage_with_triggers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create attacker with "deals combat damage to player" trigger
        let attacker_id = create_creature(&mut game, "Ninja", alice, 2, 2);
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::this_deals_combat_damage_to_player(),
                vec![Effect::draw(1)],
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Generate triggers
        let mut trigger_queue = TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        // Should have triggered
        assert!(!trigger_queue.is_empty());
    }

    // === Full Game Flow Integration Test ===

    #[test]
    fn test_full_game_lightning_bolt_wins() {
        use crate::cards::definitions::{basic_mountain, lightning_bolt};
        use crate::mana::ManaSymbol;

        // Create a game with 2 players at 3 life (so Lightning Bolt is lethal)
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up for main phase (when spells can be cast)
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create Mountain on Alice's battlefield (using CardDefinition for abilities)
        let mountain = basic_mountain();
        let mountain_id = game.create_object_from_definition(&mountain, alice, Zone::Battlefield);

        // Remove summoning sickness from Mountain (it's a land)
        game.remove_summoning_sickness(mountain_id);

        // Create Lightning Bolt in Alice's hand
        let bolt = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Hand);

        // Verify initial state
        assert_eq!(game.player(alice).unwrap().life, 3);
        assert_eq!(game.player(bob).unwrap().life, 3);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);

        // Step 1: Activate Mountain's mana ability to add {R}
        // Find the mana ability index
        let mountain_obj = game.object(mountain_id).unwrap();
        let _mana_ability_index = mountain_obj
            .abilities
            .iter()
            .position(|a| a.is_mana_ability())
            .expect("Mountain should have a mana ability");

        // Tap mountain for red mana
        game.tap(mountain_id);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Verify mana was added
        assert_eq!(
            game.player(alice)
                .unwrap()
                .mana_pool
                .amount(ManaSymbol::Red),
            1
        );

        // Step 2: Cast Lightning Bolt targeting Bob
        // Move Lightning Bolt from hand to stack
        let stack_bolt_id = game.move_object(bolt_id, Zone::Stack).unwrap();

        // Create stack entry with Bob as target
        let entry = StackEntry::new(stack_bolt_id, alice).with_targets(vec![Target::Player(bob)]);
        game.push_to_stack(entry);

        // Pay the mana cost (remove red mana from pool)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .remove(ManaSymbol::Red, 1);

        // Verify spell is on stack
        assert!(!game.stack_is_empty());

        // Step 3: Resolve the stack (both players pass priority)
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_ok(), "Stack resolution should succeed");

        // Verify Lightning Bolt dealt 3 damage to Bob
        assert_eq!(game.player(bob).unwrap().life, 0);

        // Lightning Bolt should be in graveyard
        assert!(game.stack_is_empty());
        let alice_graveyard = &game.player(alice).unwrap().graveyard;
        assert_eq!(alice_graveyard.len(), 1);

        // Step 4: Check state-based actions - Bob should lose
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Bob should have lost the game
        assert!(
            game.player(bob).unwrap().has_lost,
            "Bob should have lost the game with 0 life"
        );
    }

    #[test]
    fn test_full_game_with_decision_maker() {
        use crate::cards::definitions::{basic_mountain, fireball};
        use crate::decision::DecisionMaker;

        #[derive(Debug)]
        struct TestResponseDecisionMaker {
            responses: Vec<PriorityResponse>,
            index: usize,
        }

        impl TestResponseDecisionMaker {
            fn new(responses: Vec<PriorityResponse>) -> Self {
                Self {
                    responses,
                    index: 0,
                }
            }
        }

        impl DecisionMaker for TestResponseDecisionMaker {
            fn decide_priority(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::PriorityContext,
            ) -> LegalAction {
                if self.index < self.responses.len()
                    && let PriorityResponse::PriorityAction(action) = &self.responses[self.index]
                {
                    self.index += 1;
                    return action.clone();
                }
                ctx.legal_actions
                    .iter()
                    .find(|a| matches!(a, LegalAction::PassPriority))
                    .cloned()
                    .unwrap_or_else(|| ctx.legal_actions[0].clone())
            }

            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                if self.index < self.responses.len() {
                    if let PriorityResponse::XValue(x) = self.responses[self.index] {
                        self.index += 1;
                        return x;
                    }
                    if let PriorityResponse::NumberChoice(n) = self.responses[self.index] {
                        self.index += 1;
                        return n;
                    }
                }
                ctx.min
            }

            fn decide_targets(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::TargetsContext,
            ) -> Vec<Target> {
                if self.index < self.responses.len()
                    && let PriorityResponse::Targets(targets) = &self.responses[self.index]
                {
                    self.index += 1;
                    return targets.clone();
                }
                ctx.requirements
                    .iter()
                    .filter(|r| r.min_targets > 0)
                    .filter_map(|r| r.legal_targets.first().cloned())
                    .collect()
            }
        }

        // Create a game with 2 players at 3 life
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up for main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create 4 Mountains on Alice's battlefield (Fireball with X=3 costs {3}{R} = 4 mana)
        let mountain_def = basic_mountain();
        let mut mountain_ids = Vec::new();
        for _ in 0..4 {
            let mountain_id =
                game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
            game.remove_summoning_sickness(mountain_id);
            mountain_ids.push(mountain_id);
        }

        // Create Fireball in Alice's hand
        let fireball_def = fireball();
        let fireball_id = game.create_object_from_definition(&fireball_def, alice, Zone::Hand);

        // Find mana ability index (same for all mountains)
        let mana_ability_index = game
            .object(mountain_ids[0])
            .unwrap()
            .abilities
            .iter()
            .position(|a| a.is_mana_ability())
            .expect("Mountain should have a mana ability");

        // Create scripted responses:
        // 1-4. Alice activates mana ability on each mountain (adds 4R to pool)
        // 5. Alice casts Fireball (prompts for X value since it has X in cost)
        // 6. Alice chooses X=3 (deals 3 damage)
        // 7. Alice selects Bob as target
        // 8. Bob passes priority
        // 9. Alice passes priority (spell resolves, dealing 3 damage to Bob)
        let mut responses = Vec::new();

        // Tap all 4 mountains for mana
        for &mountain_id in &mountain_ids {
            responses.push(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility {
                    source: mountain_id,
                    ability_index: mana_ability_index,
                },
            ));
        }

        // Cast Fireball
        responses.push(PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fireball_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }));

        // Choose X=3 (after CastSpell triggers ChooseXValue decision)
        responses.push(PriorityResponse::XValue(3));

        // Choose Bob as target (after X value triggers ChooseTargets decision)
        responses.push(PriorityResponse::Targets(vec![Target::Player(bob)]));

        // Both players pass priority
        responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Bob passes
        responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Alice passes

        let mut decision_maker = TestResponseDecisionMaker::new(responses);
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        // Run the decision-based priority loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 20 {
                panic!("Too many iterations - decision loop may be stuck");
            }

            // Advance to get next decision
            let progress = advance_priority(&mut game, &mut trigger_queue)
                .expect("advance_priority should not fail");

            // Helper closure to handle a decision and any nested decisions
            let handle_result = |mut result: GameProgress,
                                 game: &mut GameState,
                                 trigger_queue: &mut TriggerQueue,
                                 state: &mut PriorityLoopState,
                                 dm: &mut TestResponseDecisionMaker|
             -> Option<GameProgress> {
                loop {
                    match result {
                        GameProgress::Continue => return Some(GameProgress::Continue),
                        GameProgress::GameOver(r) => return Some(GameProgress::GameOver(r)),
                        GameProgress::StackResolved => return Some(GameProgress::StackResolved),
                        GameProgress::NeedsDecisionCtx(ctx) => {
                            result = apply_decision_context_with_dm(
                                game,
                                trigger_queue,
                                state,
                                &ctx,
                                dm,
                            )
                            .expect("apply_decision_context_with_dm should not fail");
                        }
                    }
                }
            };

            match progress {
                GameProgress::NeedsDecisionCtx(ctx) => {
                    // Apply the response
                    let result = apply_decision_context_with_dm(
                        &mut game,
                        &mut trigger_queue,
                        &mut state,
                        &ctx,
                        &mut decision_maker,
                    )
                    .expect("apply_decision_context_with_dm should not fail");

                    // Handle any nested decisions
                    if let Some(final_result) = handle_result(
                        result,
                        &mut game,
                        &mut trigger_queue,
                        &mut state,
                        &mut decision_maker,
                    ) {
                        match final_result {
                            GameProgress::GameOver(r) => {
                                assert!(
                                    matches!(r, GameResult::Winner(winner) if winner == alice),
                                    "Alice should win (Bob at 0 life)"
                                );
                                break;
                            }
                            GameProgress::Continue => break,
                            GameProgress::StackResolved => {} // Continue outer loop
                            _ => {}
                        }
                    }
                }
                GameProgress::Continue => {
                    // Phase ended - in a full game we'd continue, but for this test we're done
                    break;
                }
                GameProgress::GameOver(result) => {
                    // Game ended
                    assert!(
                        matches!(result, GameResult::Winner(winner) if winner == alice),
                        "Alice should win (Bob at 0 life)"
                    );
                    break;
                }
                GameProgress::StackResolved => {
                    // Stack resolved, continue loop to re-advance priority
                }
            }
        }

        // Verify final state
        assert_eq!(game.player(bob).unwrap().life, 0, "Bob should be at 0 life");
        assert!(
            game.player(bob).unwrap().has_lost,
            "Bob should have lost the game"
        );
    }

    // ============================================================================
    // Card-Specific Integration Tests
    // ============================================================================

    #[test]
    fn test_darksteel_colossus_shuffle_into_library() {
        use crate::cards::definitions::darksteel_colossus;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Darksteel Colossus on battlefield
        let colossus_def = darksteel_colossus();
        let colossus_id =
            game.create_object_from_definition(&colossus_def, alice, Zone::Battlefield);

        // Verify it has the ShuffleIntoLibraryFromGraveyard ability
        let colossus = game.object(colossus_id).unwrap();
        let has_ability = colossus.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
            } else {
                false
            }
        });
        assert!(
            has_ability,
            "Darksteel Colossus should have ShuffleIntoLibraryFromGraveyard"
        );

        // Record initial library size
        let _initial_library_size = game.player(alice).unwrap().library.len();

        // Verify it's on battlefield
        assert!(game.battlefield.contains(&colossus_id));
        assert_eq!(game.object(colossus_id).unwrap().zone, Zone::Battlefield);

        // Note: The actual zone change interception would happen in move_object
        // This test verifies the ability is present; full behavior would require
        // implementing the replacement effect handling in game_state.rs
    }

    #[test]
    fn test_thorn_elemental_has_ability() {
        use crate::cards::definitions::thorn_elemental;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Thorn Elemental on battlefield
        let thorn_def = thorn_elemental();
        let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

        // Verify it has trample
        let thorn = game.object(thorn_id).unwrap();
        let has_trample = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.has_trample()
            } else {
                false
            }
        });
        assert!(has_trample, "Thorn Elemental should have trample");

        // Verify it has MayAssignDamageAsUnblocked
        let has_unblocked_ability = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
            } else {
                false
            }
        });
        assert!(
            has_unblocked_ability,
            "Thorn Elemental should have MayAssignDamageAsUnblocked"
        );
    }

    #[test]
    fn test_thorn_elemental_combat_decision() {
        use crate::cards::definitions::thorn_elemental;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Thorn Elemental on battlefield
        let thorn_def = thorn_elemental();
        let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

        // Create a blocker
        let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

        // Remove summoning sickness
        game.remove_summoning_sickness(thorn_id);

        // Set up combat: Thorn Elemental attacks Bob, Blocker blocks
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: thorn_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(thorn_id, vec![blocker_id]);

        // Verify the thorn elemental has the ability that would trigger the decision
        let thorn = game.object(thorn_id).unwrap();
        let has_ability = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
            } else {
                false
            }
        });
        assert!(has_ability);

        // Without the decision (normal combat), damage goes to blocker
        // With trample, Thorn Elemental deals 7 damage: 2 to blocker (lethal), 5 to Bob
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Verify damage was dealt (trample behavior)
        assert!(!events.is_empty());
        // Blocker takes lethal damage (2)
        assert_eq!(game.damage_on(blocker_id), 2);
        // Bob takes trample damage (7 - 2 = 5)
        assert_eq!(game.player(bob).unwrap().life, 15);
    }

    #[test]
    fn test_stormbreath_dragon_has_abilities() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::stormbreath_dragon;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        let dragon = game.object(dragon_id).unwrap();

        // Verify flying
        let has_flying = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flying()
            } else {
                false
            }
        });
        assert!(has_flying, "Stormbreath Dragon should have flying");

        // Verify haste
        let has_haste = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        });
        assert!(has_haste, "Stormbreath Dragon should have haste");

        // Verify protection from white
        let has_protection = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_protection()
            } else {
                false
            }
        });
        assert!(
            has_protection,
            "Stormbreath Dragon should have protection from white"
        );

        // Verify activated ability (monstrosity)
        let has_activated = dragon
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(
            has_activated,
            "Stormbreath Dragon should have monstrosity activated ability"
        );

        // Verify triggered ability (when becomes monstrous)
        let has_triggered = dragon
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(
            has_triggered,
            "Stormbreath Dragon should have 'becomes monstrous' trigger"
        );
    }

    #[test]
    fn test_stormbreath_dragon_is_monstrous_field() {
        use crate::cards::definitions::stormbreath_dragon;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify is_monstrous starts false
        assert!(
            !game.is_monstrous(dragon_id),
            "Dragon should not be monstrous initially"
        );

        // Manually set monstrous (simulating effect execution)
        game.set_monstrous(dragon_id);

        // Verify it's now monstrous
        assert!(
            game.is_monstrous(dragon_id),
            "Dragon should be monstrous after being set"
        );
    }

    #[test]
    fn test_stormbreath_dragon_trigger_condition() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::stormbreath_dragon;
        use crate::events::other::BecameMonstrousEvent;
        use crate::triggers::check_triggers;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify the trigger condition is ThisBecomesMonstrous
        let dragon = game.object(dragon_id).unwrap();
        let has_monstrous_trigger = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("monstrous")
            } else {
                false
            }
        });
        assert!(
            has_monstrous_trigger,
            "Stormbreath Dragon should have ThisBecomesMonstrous trigger"
        );

        // Simulate the BecameMonstrous event
        let event = TriggerEvent::new(BecameMonstrousEvent::new(dragon_id, alice, 3));

        // Check if triggers fire
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "BecameMonstrous should trigger Stormbreath Dragon's ability"
        );
    }

    #[test]
    fn test_geist_of_saint_traft_has_abilities() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Geist on battlefield
        let geist_def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

        let geist = game.object(geist_id).unwrap();

        // Verify hexproof
        let has_hexproof = geist.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_hexproof()
            } else {
                false
            }
        });
        assert!(has_hexproof, "Geist should have hexproof");

        // Verify attack trigger
        let has_attack_trigger = geist.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("attacks")
            } else {
                false
            }
        });
        assert!(
            has_attack_trigger,
            "Geist should have 'when this attacks' trigger"
        );
    }

    #[test]
    fn test_geist_of_saint_traft_attack_trigger() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;
        use crate::triggers::{AttackEventTarget, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Geist on battlefield
        let geist_def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

        // Remove summoning sickness
        game.remove_summoning_sickness(geist_id);

        // Simulate the attack event
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            geist_id,
            AttackEventTarget::Player(bob),
        ));

        // Check if triggers fire
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "Attacking with Geist should trigger its ability"
        );

        // Verify the trigger creates a token with modifications
        let geist = game.object(geist_id).unwrap();
        let trigger = geist.abilities.iter().find(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("attacks")
            } else {
                false
            }
        });
        assert!(trigger.is_some());

        if let Some(ability) = trigger {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                // Verify the effect creates a token
                assert!(!triggered.effects.is_empty());
                let has_token_effect = triggered
                    .effects
                    .iter()
                    .any(|e| format!("{:?}", e).contains("CreateToken"));
                assert!(
                    has_token_effect,
                    "Geist's trigger should create a token with modifications"
                );
            }
        }
    }

    #[test]
    fn test_geist_token_has_correct_modifications() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;

        let geist_def = geist_of_saint_traft();

        // Find the triggered ability
        let trigger = geist_def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(trigger.is_some());

        if let Some(ability) = trigger {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                // Find the token creation effect
                let token_effect = triggered
                    .effects
                    .iter()
                    .find(|e| format!("{:?}", e).contains("CreateToken"));
                assert!(
                    token_effect.is_some(),
                    "Should have a token creation effect"
                );

                // The actual token properties are tested via integration tests
                // that create the token and verify its characteristics
            }
        }
    }

    #[test]
    fn test_stormbreath_dragon_monstrosity_adds_counters() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify initial state: not monstrous, no +1/+1 counters
        assert!(!game.is_monstrous(dragon_id));
        {
            let dragon = game.object(dragon_id).unwrap();
            assert_eq!(dragon.counters.get(&CounterType::PlusOnePlusOne), None);
            assert_eq!(dragon.power(), Some(4));
            assert_eq!(dragon.toughness(), Some(4));
        }

        // Execute the Monstrosity 3 effect
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Verify result indicates monstrosity was applied
        assert!(matches!(
            result.result,
            EffectResult::MonstrosityApplied { creature, n } if creature == dragon_id && n == 3
        ));

        // Verify dragon is now monstrous with 3 +1/+1 counters
        assert!(game.is_monstrous(dragon_id), "Dragon should be monstrous");
        let dragon = game.object(dragon_id).unwrap();
        assert_eq!(
            dragon.counters.get(&CounterType::PlusOnePlusOne),
            Some(&3),
            "Dragon should have 3 +1/+1 counters"
        );
        // 4/4 + 3 counters = 7/7
        assert_eq!(dragon.power(), Some(7));
        assert_eq!(dragon.toughness(), Some(7));
    }

    #[test]
    fn test_stormbreath_dragon_monstrosity_only_works_once() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Execute monstrosity once
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Verify it worked
        assert!(game.is_monstrous(dragon_id));
        assert_eq!(
            game.object(dragon_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&3)
        );

        // Try to execute monstrosity again
        let mut ctx2 = ExecutionContext::new_default(dragon_id, alice);
        let result = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

        // Should return Count(0) - nothing happened
        assert_eq!(
            result.result,
            EffectResult::Count(0),
            "Second monstrosity should do nothing"
        );

        // Counters should still be 3 (not 6)
        assert_eq!(
            game.object(dragon_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&3),
            "Counters should not have increased"
        );
    }

    #[test]
    fn test_stormbreath_dragon_becomes_monstrous_trigger_fires() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::Effect;
        use crate::events::other::BecameMonstrousEvent;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::triggers::check_triggers;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Execute monstrosity
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Now simulate the BecameMonstrous event (which would be generated by the game loop)
        let event = TriggerEvent::new(BecameMonstrousEvent::new(dragon_id, alice, 3));

        // Check if the dragon's "becomes monstrous" trigger fires
        let triggers = check_triggers(&game, &event);

        assert_eq!(
            triggers.len(),
            1,
            "Stormbreath Dragon's 'becomes monstrous' trigger should fire"
        );

        // Verify the trigger is from the dragon
        assert_eq!(triggers[0].source, dragon_id);
        assert_eq!(triggers[0].controller, alice);
    }

    // =========================================================================
    // Integration Tests for New Features
    // =========================================================================

    #[test]
    fn test_once_per_turn_ability_tracking() {
        // Test that OncePerTurn abilities can only be activated once per turn
        use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a permanent with a OncePerTurn activated ability
        let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);

        // Add a OncePerTurn activated ability (e.g., "{T}: Draw a card")
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::free(),
                        vec![Effect::tap_source()],
                    ),
                    effects: vec![Effect::draw(1)],
                    choices: vec![],
                    timing: ActivationTiming::OncePerTurn,
                    additional_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });

        // Remove summoning sickness
        game.remove_summoning_sickness(creature_id);

        // Verify the ability hasn't been activated this turn
        assert!(!game.ability_activated_this_turn(creature_id, 0));

        // Record the activation
        game.record_ability_activation(creature_id, 0);

        // Verify the ability is now tracked as activated
        assert!(game.ability_activated_this_turn(creature_id, 0));

        // Simulate next turn - tracking should be cleared
        game.next_turn();
        assert!(!game.ability_activated_this_turn(creature_id, 0));
    }

    #[test]
    fn test_protection_from_permanents_blocking() {
        use crate::ability::ProtectionFrom;
        use crate::rules::combat::can_block;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create attacker with protection from green creatures
        let attacker_id = create_creature(&mut game, "Protected Attacker", alice, 2, 2);
        let green_filter = ObjectFilter {
            colors: Some(crate::color::ColorSet::GREEN),
            card_types: vec![CardType::Creature],
            ..Default::default()
        };
        game.object_mut(attacker_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(ProtectionFrom::Permanents(
                    green_filter,
                )),
            ));

        // Create a green creature blocker
        let green_blocker_id = create_creature(&mut game, "Green Blocker", bob, 2, 2);
        game.object_mut(green_blocker_id).unwrap().color_override =
            Some(crate::color::ColorSet::GREEN);

        // Create a red creature blocker
        let red_blocker_id = create_creature(&mut game, "Red Blocker", bob, 2, 2);
        game.object_mut(red_blocker_id).unwrap().color_override = Some(crate::color::ColorSet::RED);

        let attacker = game.object(attacker_id).unwrap();
        let green_blocker = game.object(green_blocker_id).unwrap();
        let red_blocker = game.object(red_blocker_id).unwrap();

        // Green creature should NOT be able to block (protection)
        assert!(
            !can_block(attacker, green_blocker, &game),
            "Green creature should not be able to block creature with protection from green creatures"
        );

        // Red creature SHOULD be able to block
        assert!(
            can_block(attacker, red_blocker, &game),
            "Red creature should be able to block creature with protection from green creatures"
        );
    }

    #[test]
    fn test_cleanup_discard_decision() {
        use crate::turn::{apply_cleanup_discard, get_cleanup_discard_spec};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 9 cards to hand (2 over max hand size of 7)
        for i in 0..9 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        assert_eq!(game.player(alice).unwrap().hand.len(), 9);

        // Get the discard spec
        let result = get_cleanup_discard_spec(&game);
        assert!(result.is_some());

        let (player, spec) = result.unwrap();
        assert_eq!(player, alice);
        assert_eq!(spec.count, 2);
        assert_eq!(spec.hand.len(), 9);

        // Simulate player choosing specific cards to discard
        let cards_to_discard = vec![spec.hand[0], spec.hand[1]];
        let mut dm = crate::decision::AutoPassDecisionMaker;
        apply_cleanup_discard(&mut game, &cards_to_discard, &mut dm);

        // Verify hand size is now 7
        assert_eq!(game.player(alice).unwrap().hand.len(), 7);
        // Verify graveyard has 2 cards
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_legend_rule_decision() {
        use crate::rules::state_based::{apply_legend_rule_choice, get_legend_rule_specs};
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Get legend rule specs
        let specs = get_legend_rule_specs(&game);
        assert_eq!(specs.len(), 1, "Should have one legend rule spec");

        let (player, spec) = &specs[0];
        assert_eq!(*player, alice);
        assert_eq!(spec.name, "Isamaru, Hound of Konda");
        assert_eq!(spec.legends.len(), 2);

        // Player chooses to keep the first legend
        apply_legend_rule_choice(&mut game, legend1_id);

        // Verify only one legend remains on battlefield
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.battlefield.contains(&legend1_id));

        // The second legend should be in graveyard (with new ID due to zone change)
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_may_effect_with_callback() {
        use crate::decision::DecisionMaker;
        use crate::effect::EffectResult;
        use crate::executor::ExecutionContext;

        // A decision maker that always accepts May effects
        struct AcceptMayDecisionMaker;
        impl DecisionMaker for AcceptMayDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }

        // A decision maker that always declines May effects
        struct DeclineMayDecisionMaker;
        impl DecisionMaker for DeclineMayDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                false
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add some cards to library so draw can succeed
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let source_id = create_creature(&mut game, "Source", alice, 2, 2);
        let initial_hand_size = game.player(alice).unwrap().hand.len();

        let effect = Effect::may_single(Effect::draw(1));

        // Test 1: May effect with decision maker that accepts
        let mut accept_dm = AcceptMayDecisionMaker;
        let mut ctx =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut accept_dm);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Effect should have been executed (not declined)
        assert!(
            !matches!(result.result, EffectResult::Declined),
            "Effect should not be declined when decision maker accepts"
        );
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            initial_hand_size + 1,
            "Should have drawn a card"
        );

        // Test 2: May effect with decision maker that declines
        let mut decline_dm = DeclineMayDecisionMaker;
        let mut ctx2 =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut decline_dm);

        let result2 = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

        // Effect should have been declined
        assert!(
            matches!(result2.result, EffectResult::Declined),
            "Effect should be declined when decision maker declines"
        );
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            initial_hand_size + 1,
            "Should NOT have drawn another card"
        );

        // Test 3: May effect with AutoPassDecisionMaker (auto-decline)
        let mut autopass_dm = AutoPassDecisionMaker;
        let mut ctx3 =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut autopass_dm);
        let result3 = execute_effect(&mut game, &effect, &mut ctx3).unwrap();

        assert!(
            matches!(result3.result, EffectResult::Declined),
            "Effect should be auto-declined with AutoPassDecisionMaker"
        );
    }

    #[test]
    fn test_undying_trigger_generation() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Undying (now a triggered ability)
        let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::undying(),
                    effects: undying_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Undying".to_string()),
            });

        // Create a snapshot of the creature (no +1/+1 counters)
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot qualifies for undying
        assert!(
            snapshot.qualifies_for_undying(),
            "Creature with Undying and no +1/+1 counters should qualify for undying"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should generate an undying trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::undying() }),
            "Should generate an undying trigger"
        );
    }

    #[test]
    fn test_undying_does_not_trigger_with_plus_counters() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::object::CounterType;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Undying AND +1/+1 counters
        let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::undying(),
                    effects: undying_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Undying".to_string()),
            });
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 1);

        // Create a snapshot
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot does NOT qualify for undying
        assert!(
            !snapshot.qualifies_for_undying(),
            "Creature with +1/+1 counters should NOT qualify for undying"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should NOT generate an undying trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            !triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::undying() }),
            "Should NOT generate an undying trigger when creature has +1/+1 counters"
        );
    }

    #[test]
    fn test_persist_trigger_generation() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Persist (now a triggered ability)
        let creature_id = create_creature(&mut game, "Persist Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::persist(),
                    effects: persist_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Persist".to_string()),
            });

        // Create a snapshot (no -1/-1 counters)
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot qualifies for persist
        assert!(
            snapshot.qualifies_for_persist(),
            "Creature with Persist and no -1/-1 counters should qualify for persist"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should generate a persist trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::persist() }),
            "Should generate a persist trigger"
        );
    }

    #[test]
    fn test_return_from_graveyard_with_counter_effect() {
        use crate::events::zones::ZoneChangeEvent;
        use crate::executor::ExecutionContext;
        use crate::snapshot::ObjectSnapshot;
        use crate::triggers::TriggerEvent;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and put it in the graveyard
        let creature_id = create_creature(&mut game, "Dead Creature", alice, 2, 2);

        // Take snapshot BEFORE moving (captures stable_id)
        let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        game.move_object(creature_id, Zone::Graveyard);

        // The creature now has a new ID in the graveyard
        let graveyard_id = game.player(alice).unwrap().graveyard[0];

        // Create triggering event with the snapshot
        let trigger_event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        let mut ctx = ExecutionContext::new_default(graveyard_id, alice);
        ctx.triggering_event = Some(trigger_event);
        for effect in undying_effects() {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }

        // Verify the creature is now on the battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one creature on battlefield"
        );

        // Verify graveyard is empty
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            0,
            "Graveyard should be empty"
        );

        // Verify the creature has a +1/+1 counter
        let returned_id = game.battlefield[0];
        let creature = game.object(returned_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&1),
            "Creature should have one +1/+1 counter"
        );
    }

    #[test]
    fn test_once_per_turn_in_legal_actions() {
        use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up for main phase with priority
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a creature with a OncePerTurn activated ability
        let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(TotalCost::free(), Vec::new()), // Free ability for testing
                    effects: vec![Effect::draw(1)],
                    choices: vec![],
                    timing: ActivationTiming::OncePerTurn,
                    additional_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });
        game.remove_summoning_sickness(creature_id);

        // Get legal actions - ability should be available
        let actions1 = compute_legal_actions(&game, alice);
        let can_activate1 = actions1.iter().any(|a| {
            matches!(
                a,
                LegalAction::ActivateAbility { source, .. } if *source == creature_id
            )
        });
        assert!(
            can_activate1,
            "OncePerTurn ability should be available initially"
        );

        // Simulate activating the ability
        game.record_ability_activation(creature_id, 0);

        // Get legal actions again - ability should NOT be available
        let actions2 = compute_legal_actions(&game, alice);
        let can_activate2 = actions2.iter().any(|a| {
            matches!(
                a,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == creature_id && *ability_index == 0
            )
        });
        assert!(
            !can_activate2,
            "OncePerTurn ability should NOT be available after activation"
        );
    }

    #[test]
    fn test_cleanup_discard_no_decision_when_under_limit() {
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add only 5 cards to hand (under max hand size of 7)
        for i in 0..5 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        // Get the discard spec - should be None
        let spec = get_cleanup_discard_spec(&game);
        assert!(
            spec.is_none(),
            "Should not require discard when under hand limit"
        );
    }

    #[test]
    fn test_legend_rule_no_decision_when_different_names() {
        use crate::rules::state_based::get_legend_rule_specs;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with DIFFERENT names
        let legend1_card = CardBuilder::new(CardId::from_raw(1), "Isamaru")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend2_card = CardBuilder::new(CardId::from_raw(2), "Ragavan")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        game.create_object_from_card(&legend1_card, alice, Zone::Battlefield);
        game.create_object_from_card(&legend2_card, alice, Zone::Battlefield);

        // Get legend rule specs - should be empty (different names)
        let specs = get_legend_rule_specs(&game);
        assert!(
            specs.is_empty(),
            "Should not have legend rule specs for different legendary names"
        );
    }

    // ============================================================================
    // Game Loop Integration Tests for Legend Rule and Cleanup Discard
    // ============================================================================

    /// Custom decision maker for testing legend rule choices
    struct LegendRuleDecisionMaker {
        /// Which legend to keep (index into the legends list)
        keep_index: usize,
        /// Record of decisions made
        decisions_made: Vec<String>,
    }

    impl LegendRuleDecisionMaker {
        fn new(keep_index: usize) -> Self {
            Self {
                keep_index,
                decisions_made: Vec::new(),
            }
        }
    }

    impl crate::decision::DecisionMaker for LegendRuleDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Record that a legend rule decision was made
            self.decisions_made.push(format!(
                "Legend rule for '{}' with {} options",
                ctx.description,
                ctx.candidates.len()
            ));
            // Return the legend to keep based on index
            let legal_candidates: Vec<ObjectId> = ctx
                .candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .collect();
            let keep_id = legal_candidates
                .get(
                    self.keep_index
                        .min(legal_candidates.len().saturating_sub(1)),
                )
                .copied()
                .unwrap_or_else(|| ctx.candidates[0].id);
            vec![keep_id]
        }
    }

    #[test]
    fn test_legend_rule_via_game_loop() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Verify both are on battlefield
        assert_eq!(game.battlefield.len(), 2);

        // Create a decision maker that chooses the SECOND legend to keep
        let mut dm = LegendRuleDecisionMaker::new(1);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs through the game loop - this should prompt for legend rule choice
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // Verify the decision was made
        assert_eq!(dm.decisions_made.len(), 1);
        assert!(dm.decisions_made[0].contains("Isamaru"));

        // Verify only one legend remains on battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one legend remaining"
        );

        // The SECOND legend should be the one kept (since we chose index 1)
        assert!(
            game.battlefield.contains(&legend2_id),
            "Second legend should be kept"
        );
        assert!(
            !game.battlefield.contains(&legend1_id),
            "First legend should be gone"
        );

        // First legend should be in graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            1,
            "One legend should be in graveyard"
        );
    }

    #[test]
    fn test_legend_rule_keeps_first_legend() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create three legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend3_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Verify all three are on battlefield
        assert_eq!(game.battlefield.len(), 3);

        // Create a decision maker that chooses the FIRST legend to keep
        let mut dm = LegendRuleDecisionMaker::new(0);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs through the game loop
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // Verify only one legend remains on battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one legend remaining"
        );

        // The FIRST legend should be the one kept
        assert!(
            game.battlefield.contains(&legend1_id),
            "First legend should be kept"
        );

        // Two legends should be in graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            2,
            "Two legends should be in graveyard"
        );
    }

    /// Custom decision maker for testing cleanup discard choices
    struct CleanupDiscardDecisionMaker {
        /// Which card indices to discard (from the hand list)
        discard_indices: Vec<usize>,
        /// Record of decisions made
        decisions_made: Vec<String>,
    }

    impl CleanupDiscardDecisionMaker {
        fn new(discard_indices: Vec<usize>) -> Self {
            Self {
                discard_indices,
                decisions_made: Vec::new(),
            }
        }
    }

    impl crate::decision::DecisionMaker for CleanupDiscardDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.decisions_made.push(format!(
                "Discard {} cards from hand of {}",
                ctx.min,
                ctx.candidates.len()
            ));
            // Select cards at the specified indices
            self.discard_indices
                .iter()
                .filter_map(|&idx| ctx.candidates.get(idx).map(|c| c.id))
                .take(ctx.min)
                .collect()
        }

        fn decide_priority(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            LegalAction::PassPriority
        }
    }

    #[test]
    fn test_cleanup_discard_via_game_loop() {
        use crate::decisions::make_decision;
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 10 cards to hand (3 over max hand size of 7)
        let mut card_ids = Vec::new();
        for i in 0..10 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            let obj_id = game.create_object_from_card(&card, alice, Zone::Hand);
            card_ids.push(obj_id);
        }

        assert_eq!(game.player(alice).unwrap().hand.len(), 10);

        // Create a decision maker that discards the first 3 cards
        let mut dm = CleanupDiscardDecisionMaker::new(vec![0, 1, 2]);

        // Manually run cleanup discard decision flow
        if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
            let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
        }

        // Verify the decision was made
        assert_eq!(dm.decisions_made.len(), 1);
        assert!(dm.decisions_made[0].contains("Discard 3 cards"));

        // Verify hand size is now 7
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            7,
            "Hand should have 7 cards after discard"
        );

        // Verify graveyard has 3 cards
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            3,
            "Graveyard should have 3 discarded cards"
        );

        // Verify the specific cards that were discarded (first 3)
        let graveyard = &game.player(alice).unwrap().graveyard;
        // The cards get new IDs when moving zones, so we check by name
        let discarded_names: Vec<String> = graveyard
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        // Cards 0, 1, 2 should be in graveyard
        assert!(
            discarded_names.contains(&"Card 0".to_string()),
            "Card 0 should be in graveyard"
        );
        assert!(
            discarded_names.contains(&"Card 1".to_string()),
            "Card 1 should be in graveyard"
        );
        assert!(
            discarded_names.contains(&"Card 2".to_string()),
            "Card 2 should be in graveyard"
        );
        let _ = card_ids; // Suppress unused warning
    }

    #[test]
    fn test_cleanup_discard_specific_card_choice() {
        use crate::decisions::make_decision;
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 9 cards to hand (2 over max hand size of 7)
        for i in 0..9 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        let initial_hand = game.player(alice).unwrap().hand.clone();
        assert_eq!(initial_hand.len(), 9);

        // Get the names of cards at indices 3 and 7 (the ones we'll discard)
        let card_3_name = game.object(initial_hand[3]).unwrap().name.clone();
        let card_7_name = game.object(initial_hand[7]).unwrap().name.clone();

        // Create a decision maker that discards cards at indices 3 and 7
        let mut dm = CleanupDiscardDecisionMaker::new(vec![3, 7]);

        // Run cleanup discard decision flow
        if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
            let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
        }

        // Verify hand size is now 7
        assert_eq!(game.player(alice).unwrap().hand.len(), 7);

        // Verify the correct cards were discarded by checking names in graveyard
        let graveyard_names: Vec<String> = game
            .player(alice)
            .unwrap()
            .graveyard
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        assert!(
            graveyard_names.contains(&card_3_name),
            "Card at index 3 ({}) should be in graveyard",
            card_3_name
        );
        assert!(
            graveyard_names.contains(&card_7_name),
            "Card at index 7 ({}) should be in graveyard",
            card_7_name
        );

        // Verify those cards are NOT in hand anymore
        let hand_names: Vec<String> = game
            .player(alice)
            .unwrap()
            .hand
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        assert!(
            !hand_names.contains(&card_3_name),
            "Card at index 3 should NOT be in hand"
        );
        assert!(
            !hand_names.contains(&card_7_name),
            "Card at index 7 should NOT be in hand"
        );
    }

    #[test]
    fn test_legend_rule_with_different_controllers() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create the same legendary creature for two different players
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let alice_legend = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let bob_legend = game.create_object_from_card(&legend_card, bob, Zone::Battlefield);

        // Verify both are on battlefield
        assert_eq!(game.battlefield.len(), 2);

        // Create a decision maker
        let mut dm = LegendRuleDecisionMaker::new(0);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs - legend rule should NOT apply because they have different controllers
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // No legend rule decisions should have been made
        assert_eq!(
            dm.decisions_made.len(),
            0,
            "No legend rule decisions for different controllers"
        );

        // Both legends should still be on battlefield
        assert_eq!(game.battlefield.len(), 2);
        assert!(game.battlefield.contains(&alice_legend));
        assert!(game.battlefield.contains(&bob_legend));
    }

    // ============================================================================
    // Flashback Tests
    // ============================================================================

    #[test]
    fn test_flashback_appears_in_legal_actions_from_graveyard() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase for sorcery-timing spells
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly (for flashback cost {2}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice IN GRAVEYARD
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a CastSpell action for Think Twice with Alternative casting method
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_some(),
            "Should be able to cast Think Twice with flashback from graveyard"
        );
    }

    #[test]
    fn test_flashback_not_available_from_hand() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice IN HAND
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a CastSpell action for Think Twice from hand with Normal casting
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            normal_cast.is_some(),
            "Should be able to cast Think Twice normally from hand"
        );

        // Should NOT find flashback from hand
        let flashback_from_hand = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(_),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_from_hand.is_none(),
            "Should NOT be able to use flashback from hand"
        );
    }

    #[test]
    fn test_flashback_exiles_after_resolution() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Add a card to alice's library so draw can succeed
        use crate::cards::definitions::basic_island;
        let island_def = basic_island();
        let _library_card = game.create_object_from_definition(&island_def, alice, Zone::Library);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Record initial hand size
        let initial_hand_size = game.player(alice).unwrap().hand.len();

        // Cast with flashback
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Casting with flashback should succeed");

        // Spell should be on stack now
        assert_eq!(game.stack.len(), 1, "Spell should be on stack");
        let stack_entry = &game.stack[0];
        assert_eq!(
            stack_entry.casting_method,
            CastingMethod::Alternative(0),
            "Stack entry should record flashback casting method"
        );

        // Resolve the spell
        resolve_stack_entry(&mut game).expect("Resolution should succeed");

        // Verify draw happened
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size,
            initial_hand_size + 1,
            "Should have drawn 1 card"
        );

        // Verify spell is in exile (not graveyard)
        let player = game.player(alice).unwrap();
        let in_graveyard = player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            !in_graveyard,
            "Think Twice should NOT be in graveyard after flashback"
        );

        let in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(in_exile, "Think Twice SHOULD be in exile after flashback");
    }

    #[test]
    fn test_flashback_pays_alternative_cost() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add exactly 3 blue mana (flashback cost is {2}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Cast with flashback
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Casting with flashback should succeed");

        // Verify mana was spent (flashback costs {2}{U} = 3 total, we had 3 blue)
        let mana_pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(mana_pool.blue, 0, "Should have spent all mana on flashback");
    }

    #[test]
    fn test_normal_cast_goes_to_graveyard() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 2 blue mana (normal cost is {1}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Create Think Twice in HAND
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

        // Cast normally
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Normal casting should succeed");

        // Resolve the spell
        resolve_stack_entry(&mut game).expect("Resolution should succeed");

        // Verify spell is in graveyard (not exile) after normal cast
        let player = game.player(alice).unwrap();
        let in_graveyard = player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            in_graveyard,
            "Think Twice SHOULD be in graveyard after normal cast"
        );

        let in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            !in_exile,
            "Think Twice should NOT be in exile after normal cast"
        );
    }

    #[test]
    fn test_flashback_requires_enough_mana() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add only 2 mana (flashback costs {2}{U} = 3 total)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find flashback action (not enough mana)
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    casting_method: CastingMethod::Alternative(_),
                    ..
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_none(),
            "Should NOT be able to cast with flashback without enough mana"
        );
    }

    // =========================================================================
    // Everflowing Chalice / Multikicker Tests
    // =========================================================================

    #[test]
    fn test_everflowing_chalice_no_kicks() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield with 0 kicks
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 0 kicks by running the ETB effect
        // with an ExecutionContext that has 0 kicks
        let paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect (put charge counters equal to kick count)
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 0 charge counters
        let chalice = game.object(chalice_id).unwrap();
        let charge_counters = chalice
            .counters
            .get(&CounterType::Charge)
            .copied()
            .unwrap_or(0);
        assert_eq!(
            charge_counters, 0,
            "Should have 0 charge counters with 0 kicks"
        );

        // Tap for mana - should produce 0 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            0,
            "Should produce 0 colorless mana with 0 counters"
        );
    }

    #[test]
    fn test_everflowing_chalice_one_kick() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 1 kick
        let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        paid.pay(0); // Pay multikicker once
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 1 charge counter
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&1),
            "Should have 1 charge counter with 1 kick"
        );

        // Tap for mana - should produce 1 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            1,
            "Should produce 1 colorless mana with 1 counter"
        );
    }

    #[test]
    fn test_everflowing_chalice_two_kicks() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 2 kicks
        let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        paid.pay_times(0, 2); // Pay multikicker twice
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 2 charge counters
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&2),
            "Should have 2 charge counters with 2 kicks"
        );

        // Tap for mana - should produce 2 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            2,
            "Should produce 2 colorless mana with 2 counters"
        );
    }

    #[test]
    fn test_everflowing_chalice_etb_trigger_uses_object_kick_count() {
        // This test verifies that when an ETB trigger fires, it can read
        // the kick count from the permanent that entered (not from ctx)
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Set the optional_costs_paid on the object itself (simulating what
        // resolve_stack_entry does when a permanent enters)
        {
            let chalice = game.object_mut(chalice_id).unwrap();
            let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
            paid.pay_times(0, 3); // 3 kicks
            chalice.optional_costs_paid = paid;
        }

        // Now execute the ETB effect with an EMPTY context (simulating a trigger)
        // The effect should still read the kick count from the source object
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);
        // Note: ctx.optional_costs_paid is empty, but the source object has it

        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 3 charge counters (read from source object)
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&3),
            "Should have 3 charge counters (read from object's optional_costs_paid)"
        );
    }

    // =========================================================================
    // Force of Will / Alternative Cost Tests
    // =========================================================================

    #[test]
    fn test_force_of_will_alternative_cost_available() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - alice needs something to counter
        // Put a spell on the stack that bob cast
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card in hand to exile (an Island counts as blue for this test)
        // Actually, lands are colorless. Let's use a Counterspell instead.
        use crate::cards::definitions::counterspell;
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice 20 life (default)
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find alternative cost option
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_some(),
            "Should be able to cast Force of Will with alternative cost when blue card available"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_casting_flow() {
        use crate::alternative_cast::CastingMethod;
        use crate::cards::definitions::force_of_will;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - alice needs something to counter
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card in hand to exile
        use crate::cards::definitions::counterspell;
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Verify alternative method has cost effects but no mana cost
        let fow_obj = game.object(fow_id).unwrap();
        assert_eq!(fow_obj.alternative_casts.len(), 1);
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !method.cost_effects().is_empty(),
            "Force of Will should have cost effects"
        );
        assert!(
            method.mana_cost().is_none(),
            "Force of Will alternative should NOT have a mana cost"
        );

        // Now test the casting flow
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = TriggerQueue::new();

        // Execute the CastSpell action via apply_priority_response
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "CastSpell action should succeed");

        // The result should be a target selection decision
        let progress = result.unwrap();
        match &progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => {
                // Good - now let's choose the target (Lightning Bolt)
            }
            _ => {
                panic!(
                    "Expected Targets context decision after casting Force of Will, got {:?}",
                    progress
                );
            }
        }

        // Now handle the target selection
        let pending = state.pending_cast.take().unwrap();
        let target = Target::Object(bolt_id);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let result = continue_to_mana_payment(
            &mut game,
            &mut trigger_queue,
            &mut state,
            pending,
            vec![target],
            &mut dm,
        );

        // This should NOT be a PayMana decision since there's no mana cost!
        // It should go straight to casting and then return a Priority decision
        match result {
            Ok(GameProgress::NeedsDecisionCtx(ref ctx)) => {
                // Check if this is a mana payment context
                if let crate::decisions::context::DecisionContext::SelectOptions(opts_ctx) = ctx {
                    if opts_ctx.description.contains("mana") {
                        panic!(
                            "Should NOT require mana payment for Force of Will alternative cost!"
                        );
                    }
                }
                // Other context types are acceptable (including Priority)
            }
            Ok(GameProgress::Continue) => {
                // Also acceptable
            }
            Ok(GameProgress::StackResolved) => {
                // Also acceptable
            }
            Ok(GameProgress::GameOver(_)) => {
                // Shouldn't happen but handle it
            }
            Err(e) => panic!("Error during casting: {:?}", e),
        }

        // Verify the alternative costs were paid
        // - Life should have decreased by 1
        let life = game.player(alice).unwrap().life;
        assert_eq!(life, 19, "Alice should have paid 1 life (got {})", life);

        // - The blue card should have been exiled
        // Note: move_object changes the ObjectId, so we need to look in exile
        let exiled_blue_card = game.exile.iter().any(|&id| {
            if let Some(obj) = game.object(id) {
                obj.name == "Counterspell"
            } else {
                false
            }
        });
        assert!(
            exiled_blue_card,
            "Blue card (Counterspell) should be in exile"
        );

        // - Force of Will should be on the stack
        assert!(
            game.stack.iter().any(|e| {
                if let Some(obj) = game.object(e.object_id) {
                    obj.name == "Force of Will"
                } else {
                    false
                }
            }),
            "Force of Will should be on the stack"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_not_available_without_card() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand (this is her ONLY card)
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find alternative cost option (no other blue card to exile)
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_none(),
            "Should NOT be able to use alternative cost without another blue card"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_not_available_with_only_nonblue_card() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice a non-blue card (Lightning Bolt is red)
        let red_card_def = lightning_bolt();
        let _red_card_id = game.create_object_from_definition(&red_card_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find alternative cost option (no blue card to exile)
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_none(),
            "Should NOT be able to use alternative cost with only non-blue cards"
        );
    }

    #[test]
    fn test_force_of_will_normal_cast_available_with_mana() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find normal cast option
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == fow_id
            )
        });

        assert!(
            normal_cast.is_some(),
            "Should be able to cast Force of Will normally with 3UU"
        );
    }

    #[test]
    fn test_force_of_will_both_options_available() {
        use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card (for alternative cost)
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // When both Normal and Alternative are available, only Normal should appear in actions
        // The ChooseCastingMethod decision will present both options when the spell is selected
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == fow_id
            )
        });

        let alt_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(normal_cast.is_some(), "Should be able to cast normally");
        assert!(
            alt_cast.is_none(),
            "Alternative should NOT be a separate action when Normal is also available"
        );

        // Count total CastSpell actions for Force of Will from hand
        let fow_cast_count = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    LegalAction::CastSpell {
                        spell_id,
                        from_zone: Zone::Hand,
                        ..
                    } if *spell_id == fow_id
                )
            })
            .count();
        assert_eq!(
            fow_cast_count, 1,
            "Should only have one CastSpell action for Force of Will"
        );
    }

    #[test]
    fn test_choose_casting_method_flow() {
        use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
        use crate::decision::GameProgress;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card (for alternative cost)
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Now test the ChooseCastingMethod flow
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = TriggerQueue::new();

        // Cast using Normal method - should trigger ChooseCastingMethod since both methods available
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

        // Should get a ChooseCastingMethod decision
        match result {
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            )) => {
                assert_eq!(ctx.player, alice);
                assert_eq!(ctx.source, Some(fow_id));
                assert_eq!(ctx.options.len(), 2, "Should have 2 casting method options");
                assert!(ctx.description.contains("Choose casting method"));
            }
            other => panic!(
                "Expected SelectOptions context for casting method, got {:?}",
                other
            ),
        }

        // Now choose the alternative cost (index 1)
        let method_response = PriorityResponse::CastingMethodChoice(1);
        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &method_response);

        // Should get ChooseTargets decision next (Force of Will targets a spell)
        // After targets, it will ask for card to exile
        match result {
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(ctx),
            )) => {
                assert_eq!(ctx.player, alice, "Should be alice choosing targets");
            }
            other => panic!(
                "Expected Targets context decision after method choice, got {:?}",
                other
            ),
        }
    }

    // =========================================================================
    // Underworld Breach / Granted Escape Tests
    // =========================================================================

    #[test]
    fn test_underworld_breach_grants_escape_to_graveyard_cards() {
        use crate::cards::definitions::{lightning_bolt, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Lightning Bolt in graveyard
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for escape cost)
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice enough mana to cast Lightning Bolt (R)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a GrantedEscape cast option for Lightning Bolt
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_some(),
            "Should be able to cast Lightning Bolt with granted escape from graveyard"
        );
    }

    #[test]
    fn test_underworld_breach_no_escape_without_enough_cards_to_exile() {
        use crate::cards::definitions::{lightning_bolt, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Lightning Bolt in graveyard (ONLY card)
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Only 1 card in graveyard - need 3 MORE to exile for escape
        // So escape should not be available

        // Give alice enough mana to cast Lightning Bolt (R)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option (not enough cards to exile)
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Should NOT be able to use escape without enough cards to exile"
        );
    }

    #[test]
    fn test_underworld_breach_escape_needs_3_other_cards() {
        // Regression test: with 3 cards in graveyard, you can only exile 2 OTHER cards,
        // so escape (which requires exiling 3) should NOT be available
        use crate::cards::definitions::{
            counterspell, force_of_will, think_twice, underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put 3 cards in graveyard
        let think_twice_def = think_twice();
        let fow_def = force_of_will();
        let cs_def = counterspell();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
        let _fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
        let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);

        // Give alice enough mana to cast any of these
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Escape requires exiling 3 OTHER cards - but with only 3 total,
        // each card has only 2 other cards available, so NO escape should be available
        let escape_actions: Vec<_> = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    LegalAction::CastSpell {
                        from_zone: Zone::Graveyard,
                        casting_method: CastingMethod::GrantedEscape { .. },
                        ..
                    }
                )
            })
            .collect();

        assert!(
            escape_actions.is_empty(),
            "Should NOT be able to use escape with only 3 cards in graveyard (need 3 OTHER cards). Found {} escape actions: {:?}",
            escape_actions.len(),
            escape_actions
                .iter()
                .map(|a| if let LegalAction::CastSpell { spell_id, .. } = a {
                    game.object(*spell_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                })
                .collect::<Vec<_>>()
        );

        // Flashback for Think Twice SHOULD still be available though
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });
        assert!(
            flashback_action.is_some(),
            "Think Twice's intrinsic flashback should still be available"
        );
    }

    #[test]
    fn test_underworld_breach_doesnt_grant_escape_to_lands() {
        use crate::cards::definitions::{basic_mountain, underworld_breach};
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put a land in graveyard
        let mountain_def = basic_mountain();
        let mountain_id = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for potential escape cost)
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option for the land
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == mountain_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Underworld Breach should NOT grant escape to lands"
        );
    }

    #[test]
    fn test_underworld_breach_no_escape_without_breach_on_battlefield() {
        use crate::cards::definitions::lightning_bolt;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // NO Underworld Breach on battlefield

        // Put Lightning Bolt in graveyard
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option (no Underworld Breach)
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Should NOT be able to use escape without Underworld Breach on battlefield"
        );
    }

    #[test]
    fn test_force_of_will_cannot_use_alt_cost_when_escaping() {
        // This tests a tricky interaction:
        // Force of Will has an alternative cost (pay 1 life, exile a blue card from hand)
        // Underworld Breach grants escape (pay mana cost + exile 3 cards from graveyard)
        //
        // According to MTG rules, you CANNOT combine alternative costs.
        // When casting via granted escape, you must pay the escape cost (card's mana cost + exile 3).
        // You cannot use Force of Will's own alternative cost from the graveyard.

        use crate::cards::definitions::{
            counterspell, force_of_will, lightning_bolt, underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up main phase with something on the stack to counter
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_stack_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_stack_id, bob));

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Force of Will in GRAVEYARD
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for escape cost)
        let _extra1 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _extra2 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _extra3 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice a blue card in hand (would be needed for FoW's own alternative cost)
        let cs_def = counterspell();
        let _blue_card_in_hand = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast Force of Will normally (3UU = 5 mana)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find granted escape option (from graveyard via Underworld Breach)
        let granted_escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == fow_id
            )
        });

        assert!(
            granted_escape_action.is_some(),
            "Should be able to cast Force of Will via granted escape from graveyard"
        );

        // Should NOT find Force of Will's own alternative cost from graveyard
        // (Alternative cost says "from hand", not "from graveyard")
        let fow_alt_cost_from_graveyard = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            fow_alt_cost_from_graveyard.is_none(),
            "Should NOT be able to use Force of Will's own alternative cost from graveyard - \
             alternative costs cannot be combined, and FoW's alt cost requires casting from hand"
        );

        // Also verify: no weird hybrid option that combines both costs
        // (There shouldn't be any action that lets you pay "1 life + exile blue card + exile 3 from GY")
        // This is implicitly tested by the above - we only have GrantedEscape, not Alternative(0)
    }

    #[test]
    fn test_underworld_breach_escape_works_with_4_cards() {
        // With 4 cards in graveyard, escape SHOULD be available (3 other cards to exile)
        // This tests the positive case - escape IS legal when there are enough cards
        use crate::cards::definitions::{basic_mountain, think_twice, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put 4 cards in graveyard: Think Twice + 3 others (Mountain is a land but can still be exiled)
        let think_twice_def = think_twice();
        let mountain_def = basic_mountain();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
        let _m1 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
        let _m2 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
        let _m3 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

        // Give alice enough mana for flashback (2U = 3 mana, more expensive than escape's 1U)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find Think Twice [Escape] option
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            escape_action.is_some(),
            "Think Twice [Escape] should be available with 4 cards in graveyard (3 other cards to exile)"
        );

        // Also verify Think Twice's normal Flashback is still available
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_some(),
            "Think Twice's intrinsic Flashback should also be available"
        );
    }

    #[test]
    fn test_force_of_will_escape_with_spell_on_stack() {
        // Simulates:
        // - Player 1 has Underworld Breach, 5 Islands, Force of Will + 3 cards in graveyard
        // - Player 2 casts Lightning Bolt
        // - Player 1 should be able to counter with Force of Will via Escape
        use crate::cards::definitions::{
            basic_mountain, counterspell, force_of_will, lightning_bolt, think_twice,
            underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - it's Player 2's turn, main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = bob;

        // Player 1's setup: Underworld Breach + 5 Islands on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Player 1's graveyard: Force of Will, Counterspell, Think Twice, Mountain (4 cards)
        let fow_def = force_of_will();
        let cs_def = counterspell();
        let tt_def = think_twice();
        let mtn_def = basic_mountain();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
        let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);
        let _tt_id = game.create_object_from_definition(&tt_def, alice, Zone::Graveyard);
        let _mtn_id = game.create_object_from_definition(&mtn_def, alice, Zone::Graveyard);

        // Player 2 casts Lightning Bolt targeting Player 1
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        let mut bolt_entry = StackEntry::new(bolt_id, bob);
        bolt_entry.targets = vec![Target::Player(alice)];
        game.stack.push(bolt_entry);

        // Now Player 1 has priority to respond
        game.turn.priority_player = Some(alice);

        // Give Player 1 mana to cast Force of Will (3UU = 5 mana)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);

        // Compute legal actions for Player 1
        let actions = compute_legal_actions(&game, alice);

        // Should find Force of Will [Escape] option - there's a spell on the stack to counter!
        let fow_escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == fow_id
            )
        });

        assert!(
            fow_escape_action.is_some(),
            "Force of Will [Escape] should be available when there's a spell on the stack to counter. \
             Graveyard has 4 cards (3 others to exile), and Lightning Bolt is on the stack as a legal target."
        );

        // Now actually cast Force of Will via Escape
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2); // 2 players

        // Cast the spell
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::GrantedEscape {
                source: game
                    .battlefield
                    .iter()
                    .find(|&&id| {
                        game.object(id)
                            .map(|o| o.name == "Underworld Breach")
                            .unwrap_or(false)
                    })
                    .copied()
                    .unwrap(),
                exile_count: 3,
            },
        });

        let progress =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

        // Should need to choose targets (Lightning Bolt is the only legal target)
        assert!(
            matches!(
                progress,
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(_)
                ))
            ),
            "Should prompt for targets after casting Force of Will. Got: {:?}",
            progress
        );

        // Provide the target (Lightning Bolt on stack - spells are objects)
        let targets_response = PriorityResponse::Targets(vec![Target::Object(bolt_id)]);
        let progress2 =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &targets_response);

        assert!(
            progress2.is_ok(),
            "Targeting should succeed. Got: {:?}",
            progress2
        );

        // Verify the escape cost was paid:
        // - Force of Will should now be on the stack
        // - 3 cards should have been exiled from Alice's graveyard
        // - Alice's graveyard should now have only 0 cards (FoW moved to stack, 3 exiled)

        let alice_graveyard_count = game.player(alice).unwrap().graveyard.len();
        assert_eq!(
            alice_graveyard_count, 0,
            "Alice's graveyard should be empty after casting FoW via escape (1 cast + 3 exiled). Got: {}",
            alice_graveyard_count
        );

        // Verify 3 cards were exiled
        let alice_exile_count = game
            .exile
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.owner == alice).unwrap_or(false))
            .count();
        assert_eq!(
            alice_exile_count, 3,
            "3 cards should have been exiled from Alice's graveyard for escape cost. Got: {}",
            alice_exile_count
        );

        // Verify Force of Will is on the stack
        let fow_on_stack = game.stack.iter().any(|entry| {
            game.object(entry.object_id)
                .map(|o| o.name == "Force of Will")
                .unwrap_or(false)
        });
        assert!(fow_on_stack, "Force of Will should be on the stack");
    }

    // ============================================================================
    // Affinity for Artifacts Tests
    // ============================================================================

    #[test]
    fn test_affinity_reduces_mana_cost() {
        // Frogmite costs {4} with affinity for artifacts
        // With 4 artifacts in play, it should cost {0}
        use crate::cards::definitions::frogmite;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 4 artifacts on the battlefield
        for i in 0..4 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Frogmite in hand with NO mana in pool
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Compute legal actions - Frogmite should be castable with 0 mana
        let actions = compute_legal_actions(&game, alice);

        let can_cast_frogmite = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == frogmite_id
            )
        });

        assert!(
            can_cast_frogmite,
            "Should be able to cast Frogmite for free with 4 artifacts in play"
        );

        // Verify the effective cost is 0
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            0,
            "Effective cost should be 0 with 4 artifacts"
        );
    }

    #[test]
    fn test_affinity_partial_reduction() {
        // Frogmite costs {4} with affinity for artifacts
        // With 2 artifacts in play, it should cost {2}
        use crate::cards::definitions::frogmite;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 2 artifacts on the battlefield
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Frogmite in hand
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Verify the effective cost is 2
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 with 2 artifacts"
        );

        // With only 1 mana, should NOT be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == frogmite_id
            )
        });
        assert!(
            !can_cast,
            "Should NOT be able to cast Frogmite with only 1 mana when cost is 2"
        );

        // With 2 mana, should be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == frogmite_id
            )
        });
        assert!(
            can_cast,
            "Should be able to cast Frogmite with 2 mana when cost is 2"
        );
    }

    #[test]
    fn test_affinity_only_counts_own_artifacts() {
        // Affinity only counts artifacts YOU control
        use crate::cards::definitions::frogmite;
        use crate::decision::calculate_effective_mana_cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create 2 artifacts controlled by Alice
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Alice Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Create 2 artifacts controlled by Bob (should NOT count)
        for i in 10..12 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Bob Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        }

        // Put Frogmite in Alice's hand
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Verify effective cost is 2 (only Alice's artifacts count)
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 - only Alice's 2 artifacts count, not Bob's"
        );
    }

    #[test]
    fn test_frogmite_counts_as_artifact_for_affinity_when_on_battlefield() {
        // Frogmite is an artifact creature, so once on battlefield it counts for other affinity costs
        use crate::cards::definitions::frogmite;
        use crate::decision::calculate_effective_mana_cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Put one Frogmite on the battlefield
        let frogmite_def = frogmite();
        let _battlefield_frogmite_id =
            game.create_object_from_definition(&frogmite_def, alice, Zone::Battlefield);

        // Put another Frogmite in hand
        let frogmite_in_hand_id =
            game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // The first Frogmite on battlefield should count as an artifact
        let frogmite_obj = game.object(frogmite_in_hand_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            3,
            "Effective cost should be 3 - one artifact (the other Frogmite) on battlefield"
        );
    }

    // ============================================================================
    // Delve Tests
    // ============================================================================

    #[test]
    fn test_delve_reduces_mana_cost() {
        // Treasure Cruise costs {7}{U} with Delve
        // With 7 cards in graveyard, it should cost just {U}
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 7 cards in graveyard
        for i in 0..7 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice just 1 blue mana (the {U} part)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);

        // Verify the effective cost is just {U} (mana value 1)
        let tc_obj = game.object(tc_id).unwrap();
        let base_cost = tc_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            1,
            "Effective cost should be 1 (just U) with 7 cards in graveyard to delve"
        );

        // Compute legal actions - Treasure Cruise should be castable with 1 blue mana
        let actions = compute_legal_actions(&game, alice);

        let can_cast_tc = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == tc_id
            )
        });

        assert!(
            can_cast_tc,
            "Should be able to cast Treasure Cruise with 7 cards to delve and 1 blue mana"
        );
    }

    #[test]
    fn test_delve_partial_reduction() {
        // Treasure Cruise costs {7}{U} with Delve
        // With 3 cards in graveyard, it should cost {4}{U}
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 3 cards in graveyard
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Verify effective cost is {4}{U} = 5
        let tc_obj = game.object(tc_id).unwrap();
        let base_cost = tc_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            5,
            "Effective cost should be 5 (4U) with 3 cards to delve"
        );

        // With only 3 mana (not enough), should NOT be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });
        assert!(
            !can_cast,
            "Should NOT be able to cast with only 3 mana when effective cost is 5"
        );

        // With 5 mana, should be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });
        assert!(
            can_cast,
            "Should be able to cast with 5 mana when effective cost is 5"
        );
    }

    #[test]
    fn test_delve_exiles_cards_on_cast() {
        // When casting with Delve, cards should be exiled from graveyard
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::LegalAction;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 7 cards in graveyard
        let mut gy_cards = Vec::new();
        for i in 0..7 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            let id = game.create_object_from_card(&card, alice, Zone::Graveyard);
            gy_cards.push(id);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice 1 blue mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);

        // Verify initial state
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 7);
        assert_eq!(game.exile.len(), 0);

        // Cast Treasure Cruise
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: tc_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response);
        assert!(result.is_ok(), "Casting should succeed");

        // Verify 7 cards were exiled from graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            0,
            "Graveyard should be empty after delving 7 cards"
        );
        assert_eq!(
            game.exile.len(),
            7,
            "7 cards should be in exile after delving"
        );

        // Treasure Cruise should be on the stack
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_delve_cannot_cast_without_enough_graveyard_or_mana() {
        // Treasure Cruise costs {7}{U}
        // With 0 cards in graveyard and only 3 mana, should NOT be castable
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Empty graveyard
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 0);

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice 3 mana (not enough for {7}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        // Should NOT be able to cast
        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });

        assert!(
            !can_cast,
            "Should NOT be able to cast Treasure Cruise with no graveyard and only 3 mana"
        );
    }

    #[test]
    fn test_convoke_reduces_mana_cost_with_creatures() {
        // Stoke the Flames costs {2}{R}{R} with Convoke
        // With 2 untapped creatures (one red), it should cost {1}{R}
        use crate::cards::definitions::stoke_the_flames;
        use crate::color::ColorSet;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 2 untapped creatures on battlefield (one red, one colorless)
        let red_creature = CardBuilder::new(CardId::from_raw(800), "Red Creature")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let colorless_creature = CardBuilder::new(CardId::from_raw(801), "Colorless Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let red_id = game.create_object_from_card(&red_creature, alice, Zone::Battlefield);
        let colorless_id =
            game.create_object_from_card(&colorless_creature, alice, Zone::Battlefield);

        // Mark them as not summoning sick
        game.remove_summoning_sickness(red_id);
        game.remove_summoning_sickness(colorless_id);

        // Put Stoke the Flames in hand
        let stoke_def = stoke_the_flames();
        let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

        // Give alice {1}{R} mana (red creature pays one {R}, colorless pays {1} of the {2})
        // Use Colorless for generic since Generic(1) doesn't add to pool
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Verify the effective cost is reduced
        let stoke_obj = game.object(stoke_id).unwrap();
        let base_cost = stoke_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, stoke_obj, base_cost);

        // With red creature paying {R} and colorless paying {1}, remaining should be {1}{R}
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 (1 generic + 1 red) with 2 creatures to convoke"
        );

        // Compute legal actions - Stoke should be castable
        let actions = compute_legal_actions(&game, alice);

        let can_cast_stoke = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == stoke_id
            )
        });

        assert!(
            can_cast_stoke,
            "Should be able to cast Stoke the Flames with 2 creatures to convoke and 2 mana"
        );
    }

    #[test]
    fn test_convoke_taps_creatures_on_cast() {
        // When casting with Convoke, the creatures used should be tapped
        use crate::cards::definitions::stoke_the_flames;
        use crate::color::ColorSet;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 4 red creatures (enough to pay the entire cost with Convoke)
        let mut creature_ids = Vec::new();
        for i in 0..4 {
            let creature = CardBuilder::new(CardId::new(), &format!("Red Creature {}", i))
                .card_types(vec![CardType::Creature])
                .color_indicator(ColorSet::RED)
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            creature_ids.push(id);
        }

        // Put Stoke the Flames in hand
        let stoke_def = stoke_the_flames();
        let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

        // Give alice no mana - should still be able to cast with 4 creatures
        // (2 pay generic, 2 pay red)

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        let cast_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == stoke_id
            )
        });

        assert!(
            cast_action.is_some(),
            "Should be able to cast Stoke the Flames with 4 creatures to convoke (paying all costs)"
        );

        // Cast the spell - since it requires targeting, we need to handle that
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        // Apply the cast action - this returns the ChooseTargets decision
        let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

        // The spell requires targets, so we should get a ChooseTargets decision
        if let GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) = result
        {
            // Choose bob as target - this finalizes the cast and taps creatures
            let target_response = PriorityResponse::Targets(vec![Target::Player(bob)]);
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
                .unwrap();
        } else {
            panic!("Expected ChooseTargets decision, got {:?}", result);
        }

        // Now the spell should be on the stack and creatures should be tapped
        // Check how many creatures are tapped
        let tapped_count = creature_ids
            .iter()
            .filter(|&&id| game.is_tapped(id))
            .count();

        assert!(
            tapped_count >= 2,
            "At least 2 creatures should be tapped for Convoke (tapped: {})",
            tapped_count
        );
    }

    #[test]
    fn test_convoke_colored_creatures_pay_colored_mana() {
        // Red creatures should be used to pay {R} pips first
        use crate::color::ColorSet;
        use crate::decision::{calculate_convoke_cost, get_convoke_creatures};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 2 red creatures and 2 colorless creatures
        let red1 = CardBuilder::new(CardId::from_raw(800), "Red Creature 1")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let red2 = CardBuilder::new(CardId::from_raw(801), "Red Creature 2")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let colorless1 = CardBuilder::new(CardId::from_raw(802), "Colorless Creature 1")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let colorless2 = CardBuilder::new(CardId::from_raw(803), "Colorless Creature 2")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let red1_id = game.create_object_from_card(&red1, alice, Zone::Battlefield);
        let red2_id = game.create_object_from_card(&red2, alice, Zone::Battlefield);
        let colorless1_id = game.create_object_from_card(&colorless1, alice, Zone::Battlefield);
        let colorless2_id = game.create_object_from_card(&colorless2, alice, Zone::Battlefield);

        // Mark them as not summoning sick
        for id in [red1_id, red2_id, colorless1_id, colorless2_id] {
            game.remove_summoning_sickness(id);
        }

        // Get convoke creatures
        let convoke_creatures = get_convoke_creatures(&game, alice);
        assert_eq!(
            convoke_creatures.len(),
            4,
            "Should have 4 creatures available for convoke"
        );

        // Calculate convoke cost for Stoke the Flames: {2}{R}{R}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]);

        let (creatures_to_tap, remaining_cost) = calculate_convoke_cost(&game, alice, &cost);

        // Should tap all 4 creatures: 2 red for the {R}{R}, 2 colorless for the {2}
        assert_eq!(
            creatures_to_tap.len(),
            4,
            "Should tap 4 creatures to pay the entire cost"
        );

        // Remaining cost should be empty (mana value 0)
        assert_eq!(
            remaining_cost.mana_value(),
            0,
            "Remaining cost should be 0 after tapping 4 creatures"
        );
    }

    #[test]
    fn test_convoke_summoning_sick_creatures_cannot_be_tapped() {
        // Summoning sick creatures cannot be used for Convoke (unless they have haste)
        use crate::decision::get_convoke_creatures;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 2 creatures - one summoning sick, one not
        let creature1 = CardBuilder::new(CardId::from_raw(800), "Regular Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let creature2 = CardBuilder::new(CardId::from_raw(801), "Sick Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let id1 = game.create_object_from_card(&creature1, alice, Zone::Battlefield);
        let id2 = game.create_object_from_card(&creature2, alice, Zone::Battlefield);

        // from_card sets summoning_sick to false by default, so we need to:
        // - Keep id1 as not summoning sick (can be used for convoke)
        // - Set id2 as summoning sick (cannot be used for convoke)
        game.set_summoning_sick(id2);

        // Get convoke creatures
        let convoke_creatures = get_convoke_creatures(&game, alice);

        // Should only get the non-summoning-sick creature
        assert_eq!(
            convoke_creatures.len(),
            1,
            "Only 1 creature should be available (summoning sick creatures can't convoke)"
        );
        assert_eq!(
            convoke_creatures[0].0, id1,
            "Only the non-summoning-sick creature should be available"
        );
    }

    #[test]
    fn test_improvise_reduces_mana_cost_with_artifacts() {
        // Reverse Engineer costs {3}{U}{U} with Improvise
        // With 3 untapped artifacts, it should cost just {U}{U}
        use crate::cards::definitions::reverse_engineer;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 3 untapped artifacts on battlefield
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Reverse Engineer in hand
        let re_def = reverse_engineer();
        let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

        // Give alice {U}{U} mana (3 artifacts pay the {3})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Verify the effective cost is just {U}{U} (mana value 2)
        let re_obj = game.object(re_id).unwrap();
        let base_cost = re_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, re_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 (just UU) with 3 artifacts to improvise"
        );

        // Compute legal actions - Reverse Engineer should be castable
        let actions = compute_legal_actions(&game, alice);

        let can_cast_re = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == re_id
            )
        });

        assert!(
            can_cast_re,
            "Should be able to cast Reverse Engineer with 3 artifacts to improvise and 2 blue mana"
        );
    }

    #[test]
    fn test_improvise_taps_artifacts_on_cast() {
        // When casting with Improvise, the artifacts used should be tapped
        use crate::cards::definitions::reverse_engineer;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 3 untapped artifacts
        let mut artifact_ids = Vec::new();
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
            artifact_ids.push(id);
        }

        // Put Reverse Engineer in hand
        let re_def = reverse_engineer();
        let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

        // Give alice {U}{U} mana (3 artifacts pay the {3})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        let cast_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == re_id
            )
        });

        assert!(
            cast_action.is_some(),
            "Should be able to cast Reverse Engineer"
        );

        // Cast the spell (no targets needed for draw spell)
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

        // Now the spell should be on the stack and artifacts should be tapped
        let tapped_count = artifact_ids
            .iter()
            .filter(|&&id| game.is_tapped(id))
            .count();

        assert_eq!(
            tapped_count, 3,
            "All 3 artifacts should be tapped for Improvise"
        );
    }

    #[cfg(feature = "net")]
    #[test]
    fn test_direct_finalize_trace_includes_delve_exile() {
        use crate::cards::CardDefinitionBuilder;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Build a spell with Delve + Convoke + Improvise and a generic cost.
        let spell_def = CardDefinitionBuilder::new(CardId::new(), "Trace Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
            .delve()
            .convoke()
            .improvise()
            .build();

        let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Hand);

        // Add 2 creatures for Convoke.
        for i in 0..2 {
            let creature = CardBuilder::new(CardId::new(), &format!("Convoke {}", i))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.remove_summoning_sickness(id);
        }

        // Add 2 artifacts for Improvise.
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Improvise {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Add 2 cards to graveyard for Delve.
        for i in 0..2 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard {}", i))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        let expected_delve: Vec<GameObjectId> = game
            .player(alice)
            .unwrap()
            .graveyard
            .iter()
            .map(|id| GameObjectId(id.0))
            .collect();

        let mut payment_trace = Vec::new();
        let mut trigger_queue = TriggerQueue::new();
        let mut dm = AutoPassDecisionMaker;
        let mut state = PriorityLoopState::new(game.players_in_game());

        finalize_spell_cast(
            &mut game,
            &mut trigger_queue,
            &mut state,
            spell_id,
            Zone::Hand,
            alice,
            Vec::new(),
            None,
            CastingMethod::Normal,
            OptionalCostsPaid::default(),
            None,
            Vec::new(),
            ManaPool::default(),
            Vec::new(),
            &mut payment_trace,
            false,
            spell_id,
            &mut dm,
        )
        .unwrap();

        // finalize_spell_cast no longer applies Convoke/Improvise fallback taps directly.
        // Those are now represented as pip-payment alternatives before finalize.
        assert_eq!(payment_trace.len(), 1);

        match &payment_trace[0] {
            CostStep::Payment(CostPayment::Exile { objects, from_zone }) => {
                assert_eq!(*from_zone, ZoneCode::Graveyard);
                assert_eq!(objects, &expected_delve);
            }
            other => panic!("Expected delve exile step first, got {:?}", other),
        }
    }

    #[test]
    fn test_improvise_only_pays_generic_mana() {
        // Improvise cannot pay for colored mana pips
        use crate::decision::{calculate_improvise_cost, get_improvise_artifacts};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 5 untapped artifacts (more than enough)
        for i in 0..5 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Verify artifacts are available
        let artifacts = get_improvise_artifacts(&game, alice);
        assert_eq!(artifacts.len(), 5, "Should have 5 artifacts available");

        // Calculate improvise cost for {3}{U}{U} - should only reduce the {3}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);

        let (artifacts_to_tap, remaining_cost) = calculate_improvise_cost(&game, alice, &cost);

        // Should tap 3 artifacts to pay the {3}
        assert_eq!(
            artifacts_to_tap.len(),
            3,
            "Should tap 3 artifacts to pay the generic mana"
        );

        // Remaining cost should be {U}{U} (mana value 2)
        assert_eq!(
            remaining_cost.mana_value(),
            2,
            "Remaining cost should be 2 (UU) - Improvise doesn't pay colored"
        );
    }

    #[test]
    fn test_improvise_already_tapped_artifacts_cannot_be_used() {
        // Tapped artifacts cannot be used for Improvise
        use crate::decision::get_improvise_artifacts;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 artifacts - 2 tapped, 1 untapped
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
            if i < 2 {
                game.tap(id);
            }
        }

        // Get improvise artifacts
        let artifacts = get_improvise_artifacts(&game, alice);

        // Should only get the 1 untapped artifact
        assert_eq!(
            artifacts.len(),
            1,
            "Only 1 artifact should be available (tapped artifacts can't improvise)"
        );
    }

    // =========================================================================
    // Search Library Tests (The Birth of Meletis)
    // =========================================================================

    #[test]
    fn test_search_library_finds_matching_card() {
        use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::executor::ExecutionContext;

        // Decision maker that always selects the first matching card
        struct SelectFirstDecisionMaker;
        impl DecisionMaker for SelectFirstDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Select the first legal candidate
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .take(1)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add basic Plains to library
        let plains_def = basic_plains();
        let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        // Also add some non-Plains cards to make the search interesting
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Random Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let initial_hand_size = game.player(alice).unwrap().hand.len();
        let initial_library_size = game.player(alice).unwrap().library.len();

        // Create a dummy source object for the context
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect directly
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify Plains moved to hand
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size,
            initial_hand_size + 1,
            "Should have one more card in hand"
        );

        // Verify library has one fewer card
        let final_library_size = game.player(alice).unwrap().library.len();
        assert_eq!(
            final_library_size,
            initial_library_size - 1,
            "Should have one fewer card in library"
        );

        // Verify the card in hand is a Plains
        let hand = &game.player(alice).unwrap().hand;
        let plains_in_hand = hand.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Plains" && o.subtypes.contains(&crate::types::Subtype::Plains))
                .unwrap_or(false)
        });
        assert!(plains_in_hand, "Plains should be in hand after search");
    }

    #[test]
    fn test_search_library_no_matching_cards() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::decision::DecisionMaker;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::ExecutionContext;

        // Decision maker for search (shouldn't be called if no matches)
        struct NoMatchDecisionMaker;
        impl DecisionMaker for NoMatchDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Should have no matching cards
                assert!(ctx.candidates.is_empty(), "Should have no matching cards");
                vec![]
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add only non-Plains cards to library (no basic Plains)
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Non-Plains Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let initial_hand_size = game.player(alice).unwrap().hand.len();

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect
        let mut dm = NoMatchDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should complete without error");

        // Result should indicate nothing was found
        if let Ok(outcome) = result {
            if let EffectResult::Count(n) = outcome.result {
                assert_eq!(n, 0, "Should find 0 cards when no Plains in library");
            }
        }

        // Hand size should be unchanged
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size, initial_hand_size,
            "Hand size should be unchanged when no matching cards"
        );
    }

    #[test]
    fn test_search_library_fail_to_find() {
        use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::ExecutionContext;

        // Decision maker that always chooses to "fail to find" even with matching cards
        struct FailToFindDecisionMaker;
        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Verify there ARE matching cards, but we choose not to find them
                assert!(
                    !ctx.candidates.is_empty(),
                    "Should have matching cards available"
                );
                // Return empty to "fail to find"
                vec![]
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add basic Plains to library
        let plains_def = basic_plains();
        let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        let initial_hand_size = game.player(alice).unwrap().hand.len();
        let initial_library_size = game.player(alice).unwrap().library.len();

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect with fail-to-find decision maker
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should complete without error");

        // Result should indicate nothing was found (player chose to fail)
        if let Ok(outcome) = result {
            if let EffectResult::Count(n) = outcome.result {
                assert_eq!(
                    n, 0,
                    "Should report 0 cards found when player fails to find"
                );
            }
        }

        // Hand size should be unchanged (player declined to take the Plains)
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size, initial_hand_size,
            "Hand size should be unchanged when player fails to find"
        );

        // Library size should also be unchanged (no card moved)
        let final_library_size = game.player(alice).unwrap().library.len();
        assert_eq!(
            final_library_size, initial_library_size,
            "Library size should be unchanged when player fails to find"
        );

        // Plains should still be in library
        let library = &game.player(alice).unwrap().library;
        let plains_in_library = library
            .iter()
            .any(|&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false));
        assert!(
            plains_in_library,
            "Plains should still be in library after fail to find"
        );
    }

    #[test]
    fn test_search_library_selects_specific_card() {
        use crate::cards::definitions::{basic_island, basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::executor::ExecutionContext;

        // Decision maker that selects the second matching card (if available)
        struct SelectSecondDecisionMaker;
        impl DecisionMaker for SelectSecondDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Select second card if available, otherwise first
                let legal_ids: Vec<ObjectId> = ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .collect();
                if legal_ids.len() > 1 {
                    vec![legal_ids[1]]
                } else if let Some(&id) = legal_ids.first() {
                    vec![id]
                } else {
                    vec![]
                }
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add multiple basic Plains to library
        let plains_def = basic_plains();
        let plains1_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);
        let plains2_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        // Add a non-matching card between them
        let island_def = basic_island();
        let _island_id = game.create_object_from_definition(&island_def, alice, Zone::Library);

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect
        let mut dm = SelectSecondDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify exactly one Plains moved to hand
        let hand = &game.player(alice).unwrap().hand;
        let plains_count_in_hand = hand
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
            .count();
        assert_eq!(
            plains_count_in_hand, 1,
            "Exactly one Plains should be in hand"
        );

        // Verify one Plains remains in library
        let library = &game.player(alice).unwrap().library;
        let plains_count_in_library = library
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
            .count();
        assert_eq!(
            plains_count_in_library, 1,
            "One Plains should remain in library"
        );

        // Check that one of the specific Plains IDs moved
        // (Note: IDs change on zone change, so we check by name)
        let moved_to_hand = !game.player(alice).unwrap().library.contains(&plains1_id)
            || !game.player(alice).unwrap().library.contains(&plains2_id);
        assert!(moved_to_hand, "One of the Plains should have moved to hand");
    }

    // ============================================================================
    // Saga Integration Tests
    // ============================================================================

    #[test]
    fn test_saga_etb_adds_lore_counter() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that a saga entering the battlefield gets its initial lore counter
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put saga directly on battlefield (simulating resolution)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Add initial lore counter and check chapters (what resolve_stack_entry_full does)
        add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

        // Verify saga has 1 lore counter
        let saga = game.object(saga_id).unwrap();
        let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
        assert_eq!(lore_count, 1, "Saga should have 1 lore counter after ETB");

        // Verify chapter 1 trigger is queued
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 1 trigger should be in queue"
        );
    }

    #[test]
    fn test_saga_precombat_main_adds_lore_counter() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that sagas get a lore counter at precombat main phase
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put saga on battlefield with 1 lore counter already (simulating after ETB)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 1);

        // Simulate precombat main phase - add lore counters to sagas
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify saga now has 2 lore counters
        let saga = game.object(saga_id).unwrap();
        let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
        assert_eq!(
            lore_count, 2,
            "Saga should have 2 lore counters after precombat main"
        );

        // Verify chapter 2 trigger is queued (threshold crossed from 1 to 2)
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 2 trigger should be in queue"
        );
    }

    #[test]
    fn test_saga_final_chapter_marks_for_sacrifice() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that when the final chapter ability resolves, the saga is marked for sacrifice
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put saga on battlefield with 3 lore counters (final chapter)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);

        // Verify saga is not marked as final_chapter_resolved yet
        assert!(
            !game.is_saga_final_chapter_resolved(saga_id),
            "Saga should not be marked as resolved yet"
        );

        // Simulate final chapter ability resolving by calling mark_saga_final_chapter_resolved
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Verify saga is now marked as final_chapter_resolved
        assert!(
            game.is_saga_final_chapter_resolved(saga_id),
            "Saga should be marked as resolved after final chapter ability"
        );
    }

    #[test]
    fn test_saga_sacrifice_sba() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::rules::state_based::check_state_based_actions;

        // Test that a saga marked as final_chapter_resolved is sacrificed by SBA
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put saga on battlefield with final chapter resolved
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);
        game.mark_saga_final_chapter_resolved(saga_id);

        // Verify saga is on battlefield
        assert!(
            game.battlefield.contains(&saga_id),
            "Saga should be on battlefield"
        );

        // Check SBAs - should include saga sacrifice
        let sbas = check_state_based_actions(&game);

        // Verify saga sacrifice SBA is present
        let has_saga_sacrifice = sbas.iter().any(|sba| {
            matches!(
                sba,
                crate::rules::state_based::StateBasedAction::SagaSacrifice(id) if *id == saga_id
            )
        });
        assert!(
            has_saga_sacrifice,
            "SBA should include saga sacrifice for resolved saga"
        );

        // Apply SBAs
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is no longer on battlefield
        assert!(
            !game.battlefield.contains(&saga_id),
            "Saga should no longer be on battlefield after SBA"
        );

        // Verify a saga is in graveyard (note: zone change creates new object ID per rule 400.7)
        let alice_player = game.player(alice).unwrap();
        let saga_in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "The Birth of Meletis")
                .unwrap_or(false)
        });
        assert!(
            saga_in_graveyard,
            "Saga should be in graveyard after sacrifice"
        );
    }

    #[test]
    fn test_saga_full_lifecycle() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test the full saga lifecycle: ETB -> chapter triggers -> sacrifice
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Create saga and simulate entering battlefield
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Add initial lore counter and check chapters
        add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

        // Verify: 1 lore counter, chapter 1 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            1
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Clear trigger queue (simulating triggers going on stack and resolving)
        trigger_queue.clear();

        // Simulate turn 2 - add lore counter at precombat main
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify: 2 lore counters, chapter 2 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Clear trigger queue
        trigger_queue.clear();

        // Simulate turn 3 - add lore counter at precombat main (final chapter)
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify: 3 lore counters, chapter 3 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            3
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Verify saga is NOT marked for sacrifice yet (final chapter hasn't resolved)
        assert!(!game.is_saga_final_chapter_resolved(saga_id));

        // Simulate final chapter ability resolving
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Verify saga IS marked for sacrifice
        assert!(game.is_saga_final_chapter_resolved(saga_id));

        // Apply SBAs - saga should be sacrificed
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is no longer on battlefield
        assert!(
            !game.battlefield.contains(&saga_id),
            "Saga should no longer be on battlefield"
        );

        // Verify saga is in graveyard (note: zone change creates new object ID per rule 400.7)
        let alice_player = game.player(alice).unwrap();
        let saga_in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "The Birth of Meletis")
                .unwrap_or(false)
        });
        assert!(
            saga_in_graveyard,
            "Saga should be in graveyard after sacrifice"
        );
    }

    #[test]
    fn test_saga_survives_when_lore_counter_removed() {
        use crate::cards::definitions::{hex_parasite, ornithopter, urzas_saga};
        use crate::executor::execute_effect;

        // Test that removing a lore counter from a saga at its final chapter prevents sacrifice
        // This simulates: Urza's Saga with 2 counters, gets 3rd counter (final chapter),
        // respond with Hex Parasite to remove a counter, saga survives
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put Urza's Saga on battlefield with 2 lore counters
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Put Hex Parasite on battlefield (not summoning sick for this test)
        let parasite_def = hex_parasite();
        let parasite_id =
            game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(parasite_id);

        // Put Ornithopter in library (for Urza's Saga to find)
        let ornithopter_def = ornithopter();
        let _ornithopter_id =
            game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

        // Verify initial state
        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            2,
            "Saga should start with 2 lore counters"
        );

        // Simulate precombat main phase - saga gets 3rd lore counter (final chapter)
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify saga now has 3 lore counters and chapter 3 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            3,
            "Saga should have 3 lore counters"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 3 trigger should be in queue"
        );

        // The chapter 3 trigger is now in the queue, but BEFORE it resolves,
        // we respond by activating Hex Parasite to remove a lore counter.
        // (In a real game, the trigger would go on the stack, and we'd respond)

        // Simulate Hex Parasite's ability: remove 1 lore counter from Urza's Saga
        // (Paying 2 life for the phyrexian black mana)
        let remove_effect = Effect::remove_counters(
            CounterType::Lore,
            1, // Remove 1 counter (X=1)
            ChooseSpec::SpecificObject(saga_id),
        );
        let mut ctx = ExecutionContext::new_default(parasite_id, alice)
            .with_x(1)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)]);
        let result = execute_effect(&mut game, &remove_effect, &mut ctx);
        assert!(result.is_ok(), "Counter removal should succeed");

        // Pay the life cost (2 life for phyrexian black)
        game.player_mut(alice).unwrap().life -= 2;

        // Verify saga now has 2 lore counters (not 3)
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should have 2 lore counters after Hex Parasite"
        );

        // Now the chapter 3 trigger resolves - search for artifact with MV 0 or 1
        // For this test, we'll manually resolve it
        // Create a decision maker that selects the ornithopter
        struct SelectOrnithopterDecisionMaker;
        impl DecisionMaker for SelectOrnithopterDecisionMaker {
            fn decide_objects(
                &mut self,
                game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Find ornithopter in candidates
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .find(|c| {
                        game.object(c.id)
                            .map(|o| o.name == "Ornithopter")
                            .unwrap_or(false)
                    })
                    .map(|c| vec![c.id])
                    .unwrap_or_default()
            }
        }

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Artifact],
                mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
                ..Default::default()
            },
            Zone::Battlefield,
            crate::target::PlayerFilter::You,
            false,
        );
        let mut dm = SelectOrnithopterDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Simulate the final chapter ability resolving - this would mark the saga
        // But ONLY if it still has enough lore counters
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // The saga is marked as final_chapter_resolved, but it only has 2 counters
        assert!(
            game.is_saga_final_chapter_resolved(saga_id),
            "Saga should be marked as final chapter resolved"
        );
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should still have only 2 lore counters"
        );

        // Now check SBAs - the saga should NOT be sacrificed because it doesn't have
        // enough lore counters (need 3, only has 2)
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is STILL on the battlefield
        assert!(
            game.battlefield.contains(&saga_id),
            "Saga should STILL be on battlefield - it survived because lore counter was removed!"
        );

        // Verify Ornithopter is on the battlefield (it was fetched)
        let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Ornithopter")
                .unwrap_or(false)
        });
        assert!(
            ornithopter_on_battlefield,
            "Ornithopter should be on battlefield (fetched by Urza's Saga)"
        );

        // Verify Hex Parasite is still on battlefield
        assert!(
            game.battlefield.contains(&parasite_id),
            "Hex Parasite should still be on battlefield"
        );

        // Verify Alice paid 2 life
        assert_eq!(
            game.player(alice).unwrap().life,
            18,
            "Alice should have 18 life (paid 2 for Hex Parasite)"
        );

        // Final summary of board state
        println!("Board state after Hex Parasite saves Urza's Saga:");
        println!("- Urza's Saga: on battlefield with 2 lore counters");
        println!("- Hex Parasite: on battlefield");
        println!("- Ornithopter: on battlefield (fetched)");
        println!("- Alice's life: 18");
    }

    #[test]
    fn test_saga_chapter_triggers_again_after_counter_removed() {
        use crate::cards::definitions::urzas_saga;

        // Test scenario: Hex Parasite + Urza's Saga
        // 1. Urza's Saga has 2 lore counters
        // 2. Precombat main: lore counter added (now 3), Chapter III triggers
        // 3. In response: remove a lore counter (now 2)
        // 4. Chapter III resolves (saga survives because 2 < 3)
        // 5. NEXT TURN: lore counter added (now 3), Chapter III should trigger AGAIN
        // 6. Chapter III resolves, saga gets sacrificed
        //
        // This tests MTG Rule 714.2c: chapters can trigger multiple times if the
        // threshold is crossed multiple times (e.g., by removing and re-adding counters).

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put Urza's Saga on battlefield with 2 lore counters
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Set active player to Alice (needed for add_saga_lore_counters)
        game.turn.active_player = alice;

        // --- TURN 1: Precombat main phase ---
        // Add lore counter (2 -> 3), Chapter III triggers
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            3,
            "Turn 1: Saga should have 3 lore counters after precombat main"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Turn 1: Chapter III should have triggered"
        );

        // Simulate responding with Hex Parasite: remove 1 lore counter
        game.object_mut(saga_id)
            .unwrap()
            .remove_counters(CounterType::Lore, 1);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            2,
            "Turn 1: Saga should have 2 lore counters after Hex Parasite"
        );

        // Chapter III trigger resolves (the search effect)
        // For this test, we just clear the queue to simulate resolution
        trigger_queue.clear();

        // Mark final chapter as resolved
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Check SBAs - saga should survive because 2 < 3
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
        assert!(
            game.battlefield.contains(&saga_id),
            "Turn 1: Saga should survive - only has 2 lore counters"
        );

        // --- TURN 2: Precombat main phase ---
        // Reset final_chapter_resolved for next turn's processing
        // (In a real game, this would be a new chapter trigger instance)
        game.clear_saga_final_chapter_resolved(saga_id);

        // Add lore counter (2 -> 3), Chapter III should trigger AGAIN!
        // This is the key test: the threshold crossing logic should allow re-triggering
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            3,
            "Turn 2: Saga should have 3 lore counters"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Turn 2: Chapter III should have triggered AGAIN (threshold crossed again)"
        );

        // Chapter III trigger resolves
        trigger_queue.clear();
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Check SBAs - saga should now be sacrificed because 3 >= 3
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
        assert!(
            !game.battlefield.contains(&saga_id),
            "Turn 2: Saga should be sacrificed - has 3 lore counters"
        );

        println!("Test passed: Chapter III triggered twice after counter manipulation!");
    }

    #[test]
    fn test_urzas_saga_excludes_x_cost_artifacts() {
        use crate::cards::definitions::{everflowing_chalice, ornithopter, urzas_saga};
        use crate::executor::execute_effect;
        use crate::target::FilterContext;

        // Test that Urza's Saga's search filter correctly excludes X-cost artifacts
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put Urza's Saga on battlefield
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Put Everflowing Chalice in library (has X in its cost)
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Library);

        // Put Ornithopter in library (no X in cost, mana value 0)
        let ornithopter_def = ornithopter();
        let _ornithopter_id =
            game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

        // Create the filter from Urza's Saga chapter III
        let filter = crate::target::ObjectFilter {
            card_types: vec![CardType::Artifact],
            mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
            has_mana_cost: true,
            no_x_in_cost: true,
            ..Default::default()
        };

        let ctx = FilterContext::new(alice).with_source(saga_id);

        // Verify Everflowing Chalice does NOT match (has X in cost)
        let chalice_obj = game.object(chalice_id).unwrap();
        assert!(
            !filter.matches(chalice_obj, &ctx, &game),
            "Everflowing Chalice should NOT match - has X in cost"
        );

        // Verify Ornithopter DOES match (mana value 0, no X, has mana cost)
        let ornithopter_obj = game
            .player(alice)
            .unwrap()
            .library
            .iter()
            .find_map(|&id| {
                let obj = game.object(id)?;
                if obj.name == "Ornithopter" {
                    Some(obj)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            filter.matches(ornithopter_obj, &ctx, &game),
            "Ornithopter SHOULD match - mana value 0, no X, has mana cost"
        );

        // Now test the full search effect
        struct SelectFirstMatchDecisionMaker;
        impl DecisionMaker for SelectFirstMatchDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .take(1)
                    .collect()
            }
        }

        let search_effect = Effect::search_library(
            filter,
            Zone::Battlefield,
            crate::target::PlayerFilter::You,
            false,
        );

        let mut dm = SelectFirstMatchDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify Ornithopter is on battlefield (should have been selected)
        let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Ornithopter")
                .unwrap_or(false)
        });
        assert!(
            ornithopter_on_battlefield,
            "Ornithopter should be on battlefield"
        );

        // Verify Everflowing Chalice is NOT on battlefield (should not have been searchable)
        let chalice_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Everflowing Chalice")
                .unwrap_or(false)
        });
        assert!(
            !chalice_on_battlefield,
            "Everflowing Chalice should NOT be on battlefield - has X in cost"
        );
    }

    #[test]
    fn test_hex_parasite_pump_effect() {
        use crate::cards::definitions::{hex_parasite, the_birth_of_meletis};
        use crate::executor::execute_effect;

        // Test that Hex Parasite gets +1/+0 for each counter removed
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with some lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Put Hex Parasite on battlefield
        let parasite_def = hex_parasite();
        let parasite_id =
            game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(parasite_id);

        // Verify initial state - Hex Parasite is 1/1
        let parasite = game.object(parasite_id).unwrap();
        assert_eq!(parasite.power().unwrap(), 1, "Hex Parasite base power is 1");
        assert_eq!(
            parasite.toughness().unwrap(),
            1,
            "Hex Parasite base toughness is 1"
        );

        // Execute the counter removal + pump effect sequence
        // First, remove 2 counters (X=2)
        let remove_effect = Effect::with_id(
            0,
            Effect::remove_counters(
                CounterType::Lore,
                2,
                crate::target::ChooseSpec::SpecificObject(saga_id),
            ),
        );

        let mut ctx = ExecutionContext::new_default(parasite_id, alice)
            .with_x(2)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)]);
        let result = execute_effect(&mut game, &remove_effect, &mut ctx);
        assert!(result.is_ok(), "Counter removal should succeed");

        // Check that 2 counters were removed
        assert_eq!(
            result.unwrap().as_count().unwrap_or(0),
            2,
            "Should have removed 2 counters"
        );

        // Now execute the pump effect (which uses the stored result)
        let pump_effect = Effect::if_then(
            crate::effect::EffectId(0),
            crate::effect::EffectPredicate::Happened,
            vec![Effect::pump(
                Value::EffectValue(crate::effect::EffectId(0)),
                Value::Fixed(0),
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        );

        let result = execute_effect(&mut game, &pump_effect, &mut ctx);
        assert!(result.is_ok(), "Pump effect should succeed");

        // Verify the continuous effect was added
        let effects = game.continuous_effects.effects_for_object(parasite_id);
        assert!(
            !effects.is_empty(),
            "Should have a continuous effect on Hex Parasite"
        );

        // Verify the effect is a +2/+0 modification
        let pump_effect = effects.iter().find(|e| {
            matches!(
                &e.modification,
                crate::continuous::Modification::ModifyPowerToughness {
                    power: 2,
                    toughness: 0
                }
            )
        });
        assert!(
            pump_effect.is_some(),
            "Should have a +2/+0 continuous effect"
        );
    }

    #[test]
    fn test_remove_up_to_counters_player_choice() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::executor::execute_effect;

        // Test that RemoveUpToCounters allows player to choose how many counters to remove
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with 3 lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);

        // Create a decision maker that chooses to remove only 1 counter (not the max)
        struct ChooseOneDecisionMaker;
        impl DecisionMaker for ChooseOneDecisionMaker {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                // Verify the range is correct (0 to 3, since X=5 but only 3 available)
                assert_eq!(ctx.min, 0, "Min should be 0 for 'up to' effect");
                assert_eq!(ctx.max, 3, "Max should be 3 (number available)");
                // Choose to remove only 1 counter
                1
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseOneDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(5) // Pay X=5, but only 3 counters available
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)])
            .with_decision_maker(&mut dm);

        // Use RemoveUpToCounters - player should be able to choose 0-3
        let effect = Effect::remove_up_to_counters(
            CounterType::Lore,
            Value::X,
            crate::target::ChooseSpec::SpecificObject(saga_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify only 1 counter was removed (player's choice)
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(
            removed, 1,
            "Should have removed exactly 1 counter (player's choice)"
        );

        // Verify saga still has 2 lore counters
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should have 2 lore counters remaining"
        );
    }

    #[test]
    fn test_remove_up_to_counters_choose_zero() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::executor::execute_effect;

        // Test that player can choose to remove 0 counters with "up to" effect
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with 2 lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Create a decision maker that chooses to remove 0 counters
        struct ChooseZeroDecisionMaker;
        impl DecisionMaker for ChooseZeroDecisionMaker {
            fn decide_number(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                // Choose to remove 0 counters
                0
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseZeroDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(3)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)])
            .with_decision_maker(&mut dm);

        let effect = Effect::remove_up_to_counters(
            CounterType::Lore,
            Value::X,
            crate::target::ChooseSpec::SpecificObject(saga_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 0 counters were removed
        let removed = result.unwrap().as_count().unwrap_or(-1);
        assert_eq!(
            removed, 0,
            "Should have removed 0 counters (player's choice)"
        );

        // Verify saga still has all 2 lore counters
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should still have all 2 lore counters"
        );
    }

    #[test]
    fn test_remove_up_to_any_counters_multiple_types() {
        use crate::executor::execute_effect;

        // Test that RemoveUpToAnyCounters works with multiple counter types
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a creature with multiple types of counters
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(999), "Test Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add multiple types of counters
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 3);
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::Charge, 2);

        // Verify initial state: 5 total counters
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            3
        );
        assert_eq!(
            creature
                .counters
                .get(&CounterType::Charge)
                .copied()
                .unwrap_or(0),
            2
        );

        // Create a decision maker that chooses to remove 4 counters (2 Charge + 2 +1/+1)
        struct ChooseFourDecisionMaker;
        impl DecisionMaker for ChooseFourDecisionMaker {
            fn decide_counters(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::CountersContext,
            ) -> Vec<(CounterType, u32)> {
                // max_total is capped to min(X, total_available_counters) = min(10, 5) = 5
                assert_eq!(
                    ctx.max_total, 5,
                    "Max should be capped to available counters"
                );
                assert_eq!(
                    ctx.available_counters.len(),
                    2,
                    "Should have 2 counter types"
                );
                // Choose to remove 2 Charge and 2 +1/+1 = 4 total
                vec![(CounterType::Charge, 2), (CounterType::PlusOnePlusOne, 2)]
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseFourDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(10) // X=10, but only 5 counters available
            .with_targets(vec![crate::executor::ResolvedTarget::Object(creature_id)])
            .with_decision_maker(&mut dm);

        let effect = Effect::remove_up_to_any_counters(
            Value::X,
            crate::target::ChooseSpec::SpecificObject(creature_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 4 counters were removed
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(removed, 4, "Should have removed 4 counters");

        // Verify final state: 1 +1/+1 counter remaining, 0 Charge remaining
        // (We chose to remove 2 Charge and 2 +1/+1)
        let creature = game.object(creature_id).unwrap();
        let charge_remaining = creature
            .counters
            .get(&CounterType::Charge)
            .copied()
            .unwrap_or(0);
        let plus_remaining = creature
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0);

        assert_eq!(charge_remaining, 0, "All Charge counters should be removed");
        assert_eq!(plus_remaining, 1, "Should have 1 +1/+1 counter remaining");
    }

    #[test]
    fn test_hex_parasite_removes_loyalty_counters() {
        use crate::executor::execute_effect;

        // Test that Hex Parasite can remove loyalty counters from a planeswalker
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a planeswalker with loyalty counters
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(998), "Test Planeswalker")
                .card_types(vec![CardType::Planeswalker])
                .build();
        let pw_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add loyalty counters
        game.object_mut(pw_id)
            .unwrap()
            .add_counters(CounterType::Loyalty, 4);

        // Create a decision maker that removes 2 loyalty counters
        struct ChooseTwoDecisionMaker;
        impl DecisionMaker for ChooseTwoDecisionMaker {
            fn decide_counters(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::CountersContext,
            ) -> Vec<(CounterType, u32)> {
                assert_eq!(
                    ctx.available_counters.len(),
                    1,
                    "Should only have Loyalty counters"
                );
                assert_eq!(ctx.available_counters[0].0, CounterType::Loyalty);
                // Choose to remove 2 Loyalty counters
                vec![(CounterType::Loyalty, 2)]
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseTwoDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(5)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(pw_id)])
            .with_decision_maker(&mut dm);

        // Use the same effect Hex Parasite uses
        let effect = Effect::remove_up_to_any_counters(
            Value::X,
            crate::target::ChooseSpec::SpecificObject(pw_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 2 loyalty counters were removed
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(removed, 2, "Should have removed 2 counters");

        // Verify planeswalker has 2 loyalty remaining
        let pw = game.object(pw_id).unwrap();
        assert_eq!(
            pw.counters.get(&CounterType::Loyalty).copied().unwrap_or(0),
            2,
            "Planeswalker should have 2 loyalty remaining"
        );
    }

    // ========================================================================
    // Valley Floodcaller Tests
    // ========================================================================

    #[test]
    fn test_valley_floodcaller_grants_flash_to_sorceries() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice enough mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 5);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        // Check that the sorcery has been granted flash
        let flash_ability = crate::static_abilities::StaticAbility::flash();
        let has_granted_flash = game.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash_ability,
        );
        assert!(
            has_granted_flash,
            "Valley Floodcaller should grant flash to sorceries in hand"
        );
    }

    #[test]
    fn test_valley_floodcaller_does_not_grant_flash_to_creatures() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a creature in Alice's hand
        let creature = CardBuilder::new(CardId::from_raw(100), "Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);

        // Check that the creature has NOT been granted flash
        let flash_ability = crate::static_abilities::StaticAbility::flash();
        let has_granted_flash = game.grant_registry.card_has_granted_ability(
            &game,
            creature_id,
            Zone::Hand,
            alice,
            &flash_ability,
        );
        assert!(
            !has_granted_flash,
            "Valley Floodcaller should NOT grant flash to creatures in hand"
        );
    }

    #[test]
    fn test_valley_floodcaller_flash_grant_removed_when_floodcaller_leaves() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        let flash_ability = crate::static_abilities::StaticAbility::flash();

        // Verify sorcery has flash while Floodcaller is on battlefield
        assert!(
            game.grant_registry.card_has_granted_ability(
                &game,
                sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Sorcery should have flash while Floodcaller is on battlefield"
        );

        // Remove Floodcaller from battlefield
        game.move_object(floodcaller_id, Zone::Graveyard);

        // Verify sorcery no longer has flash
        assert!(
            !game.grant_registry.card_has_granted_ability(
                &game,
                sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Sorcery should NOT have flash after Floodcaller leaves battlefield"
        );
    }

    #[test]
    fn test_valley_floodcaller_sorcery_castable_during_combat() {
        use crate::cards::definitions::valley_floodcaller;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 5);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Draw Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        // Set to combat phase (not main phase)
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);

        // Check that the sorcery can be cast during combat (has flash)
        let actions = compute_legal_actions(&game, alice);
        let can_cast_sorcery = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == sorcery_id
            )
        });

        assert!(
            can_cast_sorcery,
            "Should be able to cast sorcery during combat thanks to Valley Floodcaller granting flash"
        );
    }

    #[test]
    fn test_valley_floodcaller_only_grants_to_controller() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Valley Floodcaller on Alice's battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create sorceries in both players' hands
        let alice_sorcery = CardBuilder::new(CardId::from_raw(100), "Alice Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build();
        let alice_sorcery_id = game.create_object_from_card(&alice_sorcery, alice, Zone::Hand);

        let bob_sorcery = CardBuilder::new(CardId::from_raw(101), "Bob Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build();
        let bob_sorcery_id = game.create_object_from_card(&bob_sorcery, bob, Zone::Hand);

        let flash_ability = crate::static_abilities::StaticAbility::flash();

        // Alice's sorcery should have flash
        assert!(
            game.grant_registry.card_has_granted_ability(
                &game,
                alice_sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Alice's sorcery should have flash from her Floodcaller"
        );

        // Bob's sorcery should NOT have flash (Alice's Floodcaller doesn't grant to opponents)
        assert!(
            !game.grant_registry.card_has_granted_ability(
                &game,
                bob_sorcery_id,
                Zone::Hand,
                bob,
                &flash_ability,
            ),
            "Bob's sorcery should NOT have flash from Alice's Floodcaller"
        );
    }
}
