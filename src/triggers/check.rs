//! Trigger checking and queue management.
//!
//! This module contains the `check_triggers()` function that scans all permanents
//! for triggered abilities that match a game event.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::Effect;
use crate::ability::{AbilityKind, InterveningIfCondition, TriggeredAbility};
use crate::filter::ObjectRef;
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::Trigger;
use super::matcher_trait::TriggerContext;
use super::trigger_event::TriggerEvent;

/// Stable, structural identity for a trigger definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TriggerIdentity(pub u64);

/// A triggered ability that needs to go on the stack.
#[derive(Debug, Clone)]
pub struct TriggeredAbilityEntry {
    /// The source permanent that has the triggered ability.
    pub source: ObjectId,
    /// The controller of the triggered ability.
    pub controller: PlayerId,
    /// The triggered ability definition.
    pub ability: TriggeredAbility,
    /// The event that triggered this ability (for "intervening if" checks).
    pub triggering_event: TriggerEvent,
    /// Stable instance ID of the source (persists across zone changes).
    pub source_stable_id: StableId,
    /// Name of the source for display purposes.
    pub source_name: String,
    /// Structural identity of this trigger ability.
    pub trigger_identity: TriggerIdentity,
}

/// A delayed trigger that waits for a specific event to occur.
#[derive(Debug, Clone)]
pub struct DelayedTrigger {
    /// The trigger condition to wait for.
    pub trigger: Trigger,
    /// Effects to execute when the trigger fires.
    pub effects: Vec<Effect>,
    /// Whether this is a one-shot trigger (fires once then is removed).
    pub one_shot: bool,
    /// Optional minimum turn number before this delayed trigger can fire.
    pub not_before_turn: Option<u32>,
    /// Optional turn number after which this delayed trigger expires.
    pub expires_at_turn: Option<u32>,
    /// Specific objects this trigger targets.
    pub target_objects: Vec<ObjectId>,
    /// Optional source object to use for the triggered ability when it fires.
    /// If unset, the watched/target object is used as the source.
    pub ability_source: Option<ObjectId>,
    /// The controller of this delayed trigger.
    pub controller: PlayerId,
}

/// Queue of triggered abilities waiting to be put on the stack.
#[derive(Debug, Clone, Default)]
pub struct TriggerQueue {
    /// Pending triggered abilities.
    pub entries: Vec<TriggeredAbilityEntry>,
}

impl TriggerQueue {
    /// Create a new empty trigger queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a triggered ability to the queue.
    pub fn add(&mut self, entry: TriggeredAbilityEntry) {
        self.entries.push(entry);
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries from the queue.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Take all entries, leaving the queue empty.
    pub fn take_all(&mut self) -> Vec<TriggeredAbilityEntry> {
        std::mem::take(&mut self.entries)
    }
}

/// Compute a structural identity for a trigger ability.
pub fn compute_trigger_identity(trigger_ability: &TriggeredAbility) -> TriggerIdentity {
    let mut hasher = DefaultHasher::new();
    trigger_ability.trigger.display().hash(&mut hasher);
    trigger_ability.effects.len().hash(&mut hasher);
    trigger_ability.choices.len().hash(&mut hasher);
    trigger_ability.intervening_if.is_some().hash(&mut hasher);
    for effect in &trigger_ability.effects {
        format!("{:?}", effect).hash(&mut hasher);
    }
    for choice in &trigger_ability.choices {
        format!("{:?}", choice).hash(&mut hasher);
    }
    if let Some(condition) = &trigger_ability.intervening_if {
        format!("{:?}", condition).hash(&mut hasher);
    }
    TriggerIdentity(hasher.finish())
}

/// Compute a structural identity for a delayed trigger.
pub fn compute_delayed_trigger_identity(delayed: &DelayedTrigger) -> TriggerIdentity {
    let mut hasher = DefaultHasher::new();
    delayed.trigger.display().hash(&mut hasher);
    delayed.effects.len().hash(&mut hasher);
    delayed.one_shot.hash(&mut hasher);
    delayed.not_before_turn.hash(&mut hasher);
    delayed.expires_at_turn.hash(&mut hasher);
    delayed.controller.hash(&mut hasher);
    for effect in &delayed.effects {
        format!("{:?}", effect).hash(&mut hasher);
    }
    TriggerIdentity(hasher.finish())
}

/// Check all permanents for triggered abilities that match the given event.
///
/// Returns a list of triggered abilities that should go on the stack.
pub fn check_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
) -> Vec<TriggeredAbilityEntry> {
    let mut triggered = Vec::new();

    // Check all permanents on the battlefield
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };

        // Get calculated abilities (after continuous effects like Humility, Blood Moon)
        let calculated_abilities = game
            .calculated_characteristics(obj_id)
            .map(|c| c.abilities)
            .unwrap_or_else(|| obj.abilities.clone());

        // Check each ability on the permanent
        for ability in &calculated_abilities {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };

            if !ability.functions_in(&obj.zone) {
                continue;
            }

            let ctx = TriggerContext::for_source(obj_id, obj.controller, game);
            if trigger_ability.trigger.matches(trigger_event, &ctx) {
                let trigger_identity = compute_trigger_identity(trigger_ability);
                if let Some(ref condition) = trigger_ability.intervening_if
                    && !verify_intervening_if(
                        game,
                        condition,
                        obj.controller,
                        trigger_event,
                        obj_id,
                        Some(trigger_identity),
                    )
                {
                    continue;
                }

                triggered.push(TriggeredAbilityEntry {
                    source: obj_id,
                    controller: obj.controller,
                    ability: TriggeredAbility {
                        trigger: trigger_ability.trigger.clone(),
                        effects: trigger_ability.effects.clone(),
                        choices: trigger_ability.choices.clone(),
                        intervening_if: trigger_ability.intervening_if.clone(),
                    },
                    triggering_event: trigger_event.clone(),
                    source_stable_id: obj.stable_id,
                    source_name: obj.name.clone(),
                    trigger_identity,
                });
            }
        }
    }

    // Special-case: for "dies" zone changes, also allow triggers from the object
    // that died using its last-known information (LKI). This enables "dies"
    // triggers on sources that are no longer on the battlefield when checked.
    if trigger_event.kind() == crate::events::traits::EventKind::ZoneChange
        && let Some(zc) = trigger_event.downcast::<crate::events::zones::ZoneChangeEvent>()
        && zc.is_dies()
        && let Some(snapshot) = zc.snapshot.as_ref()
    {
        if !game.battlefield.contains(&snapshot.object_id) {
            for ability in &snapshot.abilities {
                let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                    continue;
                };

                // Only consider abilities that function on the battlefield.
                if !ability.functions_in(&Zone::Battlefield) {
                    continue;
                }

                let ctx = TriggerContext::for_source(snapshot.object_id, snapshot.controller, game);
                if trigger_ability.trigger.matches(trigger_event, &ctx) {
                    let trigger_identity = compute_trigger_identity(trigger_ability);
                    if let Some(ref condition) = trigger_ability.intervening_if
                        && !verify_intervening_if(
                            game,
                            condition,
                            snapshot.controller,
                            trigger_event,
                            snapshot.object_id,
                            Some(trigger_identity),
                        )
                    {
                        continue;
                    }

                    triggered.push(TriggeredAbilityEntry {
                        source: snapshot.object_id,
                        controller: snapshot.controller,
                        ability: TriggeredAbility {
                            trigger: trigger_ability.trigger.clone(),
                            effects: trigger_ability.effects.clone(),
                            choices: trigger_ability.choices.clone(),
                            intervening_if: trigger_ability.intervening_if.clone(),
                        },
                        triggering_event: trigger_event.clone(),
                        source_stable_id: snapshot.stable_id,
                        source_name: snapshot.name.clone(),
                        trigger_identity,
                    });
                }
            }
        }
    }

    // Check objects in other zones
    for player in &game.players {
        for &obj_id in &player.graveyard {
            check_triggers_in_zone(game, obj_id, trigger_event, &mut triggered);
        }
        for &obj_id in &player.hand {
            check_triggers_in_zone(game, obj_id, trigger_event, &mut triggered);
        }
    }

    // Check emblems in command zone
    for &obj_id in &game.command_zone {
        check_triggers_in_zone(game, obj_id, trigger_event, &mut triggered);
    }

    // Note: Undying/Persist/Miracle triggers are handled through the normal trigger system.
    // They function from the graveyard/hand (where the object is after the event) and use
    // the triggering_event to get stable_id and other context at execution time.

    triggered
}

/// Check delayed triggers against an event and return triggered entries.
pub fn check_delayed_triggers(
    game: &mut GameState,
    trigger_event: &TriggerEvent,
) -> Vec<TriggeredAbilityEntry> {
    let mut triggered = Vec::new();
    let mut to_remove = Vec::new();

    for (idx, delayed) in game.delayed_triggers.iter().enumerate() {
        if delayed
            .expires_at_turn
            .is_some_and(|max_turn| game.turn.turn_number > max_turn)
        {
            to_remove.push(idx);
            continue;
        }
        if delayed
            .not_before_turn
            .is_some_and(|min_turn| game.turn.turn_number < min_turn)
        {
            continue;
        }
        let candidate_sources = if delayed.target_objects.is_empty() {
            vec![ObjectId::from_raw(0)]
        } else {
            delayed.target_objects.clone()
        };

        let mut fired = false;
        for source in candidate_sources {
            let ctx = TriggerContext::for_source(source, delayed.controller, game);
            if !delayed.trigger.matches(trigger_event, &ctx) {
                continue;
            }

            fired = true;
            let ability_source = delayed.ability_source.unwrap_or(source);
            let source_stable_id = game
                .object(ability_source)
                .map(|o| o.stable_id)
                .or_else(|| {
                    game.find_object_by_stable_id(StableId::from(ability_source))
                        .and_then(|id| game.object(id))
                        .map(|o| o.stable_id)
                })
                .or_else(|| {
                    if trigger_event.object_id() == Some(ability_source) {
                        trigger_event.snapshot().map(|snapshot| snapshot.stable_id)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| StableId::from(ability_source));
            let source_name = game
                .object(ability_source)
                .map(|o| o.name.clone())
                .or_else(|| {
                    game.find_object_by_stable_id(source_stable_id)
                        .and_then(|id| game.object(id))
                        .map(|o| o.name.clone())
                })
                .or_else(|| {
                    if trigger_event.object_id() == Some(ability_source) {
                        trigger_event
                            .snapshot()
                            .map(|snapshot| snapshot.name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "Delayed Trigger".to_string());

            triggered.push(TriggeredAbilityEntry {
                source: ability_source,
                controller: delayed.controller,
                ability: TriggeredAbility {
                    trigger: delayed.trigger.clone(),
                    effects: delayed.effects.clone(),
                    choices: vec![],
                    intervening_if: None,
                },
                triggering_event: trigger_event.clone(),
                source_stable_id,
                source_name,
                trigger_identity: compute_delayed_trigger_identity(delayed),
            });

            if delayed.one_shot {
                break;
            }
        }

        if fired && delayed.one_shot {
            to_remove.push(idx);
        }
    }

    for idx in to_remove.into_iter().rev() {
        game.delayed_triggers.remove(idx);
    }

    triggered
}

fn check_triggers_in_zone(
    game: &GameState,
    obj_id: ObjectId,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let Some(obj) = game.object(obj_id) else {
        return;
    };

    for ability in &obj.abilities {
        let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
            continue;
        };

        if !ability.functions_in(&obj.zone) {
            continue;
        }

        let ctx = TriggerContext::for_source(obj_id, obj.controller, game);
        if trigger_ability.trigger.matches(trigger_event, &ctx) {
            let trigger_identity = compute_trigger_identity(trigger_ability);
            if let Some(ref condition) = trigger_ability.intervening_if
                && !verify_intervening_if(
                    game,
                    condition,
                    obj.controller,
                    trigger_event,
                    obj_id,
                    Some(trigger_identity),
                )
            {
                continue;
            }

            triggered.push(TriggeredAbilityEntry {
                source: obj_id,
                controller: obj.controller,
                ability: TriggeredAbility {
                    trigger: trigger_ability.trigger.clone(),
                    effects: trigger_ability.effects.clone(),
                    choices: trigger_ability.choices.clone(),
                    intervening_if: trigger_ability.intervening_if.clone(),
                },
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.clone(),
                trigger_identity,
            });
        }
    }
}

/// Check if a PlayerFilter matches a specific player, with optional combat context.
pub fn player_filter_matches_with_context(
    spec: &PlayerFilter,
    player: PlayerId,
    controller: PlayerId,
    game: &GameState,
    defending_player: Option<PlayerId>,
) -> bool {
    match spec {
        PlayerFilter::Any => true,
        PlayerFilter::You => player == controller,
        PlayerFilter::NotYou => player != controller,
        PlayerFilter::Opponent => player != controller,
        PlayerFilter::Target(_) => true,
        PlayerFilter::Specific(id) => player == *id,
        PlayerFilter::Teammate => false,
        PlayerFilter::Attacking => false,
        PlayerFilter::DamagedPlayer => false,
        PlayerFilter::ControllerOf(obj_ref) => match obj_ref {
            ObjectRef::Specific(object_id) => game
                .object(*object_id)
                .is_some_and(|obj| player == obj.controller),
            ObjectRef::Target | ObjectRef::Tagged(_) => false, // Can't resolve at trigger-check time
        },
        PlayerFilter::OwnerOf(obj_ref) => match obj_ref {
            ObjectRef::Specific(object_id) => game
                .object(*object_id)
                .is_some_and(|obj| player == obj.owner),
            ObjectRef::Target | ObjectRef::Tagged(_) => false, // Can't resolve at trigger-check time
        },
        PlayerFilter::Active => player == game.turn.active_player,
        PlayerFilter::Defending => defending_player == Some(player),
        PlayerFilter::IteratedPlayer => false,
    }
}

/// Generate phase/step trigger events based on current game state.
pub fn generate_step_trigger_events(game: &GameState) -> Option<TriggerEvent> {
    use crate::events::phase::{
        BeginningOfCombatEvent, BeginningOfDrawStepEvent, BeginningOfEndStepEvent,
        BeginningOfPostcombatMainPhaseEvent, BeginningOfPrecombatMainPhaseEvent,
        BeginningOfUpkeepEvent, EndOfCombatEvent,
    };

    let active = game.turn.active_player;

    match (game.turn.phase, game.turn.step) {
        (Phase::Beginning, Some(Step::Upkeep)) => {
            Some(TriggerEvent::new(BeginningOfUpkeepEvent::new(active)))
        }
        (Phase::Beginning, Some(Step::Draw)) => {
            Some(TriggerEvent::new(BeginningOfDrawStepEvent::new(active)))
        }
        (Phase::FirstMain, None) => Some(TriggerEvent::new(
            BeginningOfPrecombatMainPhaseEvent::new(active),
        )),
        (Phase::Combat, Some(Step::BeginCombat)) => {
            Some(TriggerEvent::new(BeginningOfCombatEvent::new(active)))
        }
        (Phase::Combat, Some(Step::EndCombat)) => Some(TriggerEvent::new(EndOfCombatEvent::new())),
        (Phase::NextMain, None) => Some(TriggerEvent::new(
            BeginningOfPostcombatMainPhaseEvent::new(active),
        )),
        (Phase::Ending, Some(Step::End)) => {
            Some(TriggerEvent::new(BeginningOfEndStepEvent::new(active)))
        }
        _ => None,
    }
}

/// Verify if an intervening-if condition is met.
pub fn verify_intervening_if(
    game: &GameState,
    condition: &InterveningIfCondition,
    controller: PlayerId,
    event: &TriggerEvent,
    source_object_id: ObjectId,
    trigger_identity: Option<TriggerIdentity>,
) -> bool {
    match condition {
        InterveningIfCondition::YouControl(filter) => {
            let ctx = game.filter_context_for(controller, None);
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    obj.controller == controller && filter.matches(obj, &ctx, game)
                })
            })
        }

        InterveningIfCondition::OpponentControls(filter) => {
            let ctx = game.filter_context_for(controller, None);
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    obj.controller != controller
                        && game
                            .player(obj.controller)
                            .map(|p| p.is_in_game())
                            .unwrap_or(false)
                        && filter.matches(obj, &ctx, game)
                })
            })
        }

        InterveningIfCondition::LifeTotalAtLeast(amount) => game
            .player(controller)
            .map(|p| p.life >= *amount)
            .unwrap_or(false),

        InterveningIfCondition::LifeTotalAtMost(amount) => game
            .player(controller)
            .map(|p| p.life <= *amount)
            .unwrap_or(false),

        InterveningIfCondition::NoCreaturesDiedThisTurn => game.creatures_died_this_turn == 0,

        InterveningIfCondition::CreatureDiedThisTurn => game.creatures_died_this_turn > 0,

        InterveningIfCondition::FirstTimeThisTurn => trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(source_object_id, id) == 0)
            .unwrap_or(true),

        InterveningIfCondition::MaxTimesEachTurn(limit) => trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(source_object_id, id) < *limit)
            .unwrap_or(true),

        InterveningIfCondition::WasEnchanted => {
            // Check if the event has a snapshot and if it was enchanted
            event.snapshot().is_some_and(|s| s.was_enchanted)
        }

        InterveningIfCondition::HadCounters(counter_type, min_count) => {
            // Check if the event has a snapshot and if it had the required counters
            event.snapshot().is_some_and(|snapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, TriggeredAbility};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::events::EventKind;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::events::zones::ZoneChangeEvent;
    use crate::ids::CardId;
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_creature_with_etb(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), "ETB Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let id = game.create_object_from_card(&card, owner, Zone::Battlefield);

        if let Some(obj) = game.object_mut(id) {
            obj.abilities.push(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::draw(1)],
            ));
        }

        id
    }

    #[test]
    fn test_trigger_queue_basic() {
        let mut queue = TriggerQueue::new();
        assert!(queue.is_empty());

        let entry = TriggeredAbilityEntry {
            source: ObjectId::from_raw(1),
            controller: PlayerId::from_index(0),
            ability: TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects: vec![Effect::draw(1)],
                choices: vec![],
                intervening_if: None,
            },
            triggering_event: TriggerEvent::new(ZoneChangeEvent::new(
                ObjectId::from_raw(1),
                Zone::Hand,
                Zone::Battlefield,
                None,
            )),
            source_stable_id: StableId::from_raw(1),
            source_name: "Test Card".to_string(),
            trigger_identity: TriggerIdentity(1),
        };

        queue.add(entry);
        assert!(!queue.is_empty());
        assert_eq!(queue.entries.len(), 1);

        let taken = queue.take_all();
        assert_eq!(taken.len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_check_triggers_etb() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = add_creature_with_etb(&mut game, alice);

        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source, creature_id);
    }

    #[test]
    fn test_check_triggers_upkeep() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Upkeep Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::draw(1)],
            ));
        }

        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(alice));
        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 1);

        let bob = PlayerId::from_index(1);
        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(bob));
        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 0);
    }

    #[test]
    fn test_generate_step_trigger_events() {
        let mut game = setup_game();
        let _alice = PlayerId::from_index(0);

        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let event = generate_step_trigger_events(&game);
        assert!(event.is_some());
        assert_eq!(event.unwrap().kind(), EventKind::BeginningOfUpkeep);
    }
}
