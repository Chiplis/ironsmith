//! Trigger checking and queue management.
//!
//! This module contains the `check_triggers()` function that scans all permanents
//! for triggered abilities that match a game event.

use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::Effect;
use crate::ability::{AbilityKind, TriggeredAbility};
use crate::continuous::ContinuousEffect;
use crate::filter::ObjectFilter;
use crate::filter::ObjectRef;
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::resolution::ResolutionProgram;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::Trigger;
use super::TriggerEvent;
use super::matcher_trait::TriggerContext;

fn trigger_entry_x_value(trigger_event: &TriggerEvent, fallback: Option<u32>) -> Option<u32> {
    trigger_event
        .downcast::<crate::events::other::BecameMonstrousEvent>()
        .map(|event| event.n)
        .or(fallback)
}

/// Stable, structural identity for a trigger definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TriggerIdentity(pub u64);

/// Stable key for remembering whether a state trigger is currently true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveStateTriggerKey {
    pub source_stable_id: StableId,
    pub trigger_identity: TriggerIdentity,
}

/// A triggered ability that needs to go on the stack.
#[derive(Debug, Clone)]
pub struct TriggeredAbilityEntry {
    /// The source permanent that has the triggered ability.
    pub source: ObjectId,
    /// The controller of the triggered ability.
    pub controller: PlayerId,
    /// X value to use when resolving this trigger (if any).
    pub x_value: Option<u32>,
    /// The triggered ability definition.
    pub ability: TriggeredAbility,
    /// The event that triggered this ability (for "intervening if" checks).
    pub triggering_event: TriggerEvent,
    /// Stable instance ID of the source (persists across zone changes).
    pub source_stable_id: StableId,
    /// Name of the source for display purposes.
    pub source_name: String,
    /// Source snapshot captured earlier when available.
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// Tagged objects captured at trigger time for delayed/tagged follow-up effects.
    pub tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    /// Structural identity of this trigger ability.
    pub trigger_identity: TriggerIdentity,
}

/// A delayed trigger that waits for a specific event to occur.
#[derive(Debug, Clone)]
pub struct DelayedTrigger {
    /// The trigger condition to wait for.
    pub trigger: Trigger,
    /// Effects to execute when the trigger fires.
    pub effects: ResolutionProgram,
    /// Whether this is a one-shot trigger (fires once then is removed).
    pub one_shot: bool,
    /// X value captured when the delayed trigger was scheduled (if any).
    pub x_value: Option<u32>,
    /// Optional minimum turn number before this delayed trigger can fire.
    pub not_before_turn: Option<u32>,
    /// Optional turn number after which this delayed trigger expires.
    pub expires_at_turn: Option<u32>,
    /// Specific objects this trigger targets.
    pub target_objects: Vec<ObjectId>,
    /// Optional source object to use for the triggered ability when it fires.
    /// If unset, the watched/target object is used as the source.
    pub ability_source: Option<ObjectId>,
    /// Stable source identity captured when the delayed trigger was scheduled.
    pub ability_source_stable_id: Option<StableId>,
    /// Source display name captured when the delayed trigger was scheduled.
    pub ability_source_name: Option<String>,
    /// Source snapshot captured when the delayed trigger was scheduled.
    pub ability_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// The controller of this delayed trigger.
    pub controller: PlayerId,
    /// Target choices for when the trigger resolves (e.g., haunt effects that target a player).
    pub choices: Vec<crate::target::ChooseSpec>,
    /// Tagged objects captured when this delayed trigger was created.
    pub tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
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
    trigger_ability
        .effects
        .all_effects()
        .len()
        .hash(&mut hasher);
    trigger_ability.choices.len().hash(&mut hasher);
    trigger_ability.intervening_if.is_some().hash(&mut hasher);
    for effect in trigger_ability.effects.all_effects() {
        let _ = crate::trigger_identity::hash_debug(&mut hasher, effect);
    }
    for choice in &trigger_ability.choices {
        let _ = crate::trigger_identity::hash_debug(&mut hasher, choice);
    }
    if let Some(condition) = &trigger_ability.intervening_if {
        let _ = crate::trigger_identity::hash_debug(&mut hasher, condition);
    }
    TriggerIdentity(hasher.finish())
}

/// Compute a structural identity for a delayed trigger.
pub fn compute_delayed_trigger_identity(delayed: &DelayedTrigger) -> TriggerIdentity {
    let mut hasher = DefaultHasher::new();
    delayed.trigger.display().hash(&mut hasher);
    delayed.effects.all_effects().len().hash(&mut hasher);
    delayed.one_shot.hash(&mut hasher);
    delayed.not_before_turn.hash(&mut hasher);
    delayed.expires_at_turn.hash(&mut hasher);
    delayed.controller.hash(&mut hasher);
    for effect in delayed.effects.all_effects() {
        let _ = crate::trigger_identity::hash_debug(&mut hasher, effect);
    }
    TriggerIdentity(hasher.finish())
}

fn battlefield_has_static_ability_with_effects(
    game: &GameState,
    ability_id: StaticAbilityId,
    all_effects: &[ContinuousEffect],
) -> bool {
    let view = crate::derived_view::DerivedGameView::from_effects(game, all_effects.to_vec());
    battlefield_has_static_ability_with_view(game, ability_id, &view)
}

fn battlefield_has_static_ability_with_view(
    game: &GameState,
    ability_id: StaticAbilityId,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    game.battlefield.iter().any(|&obj_id| {
        let Some(obj) = game.object(obj_id) else {
            return false;
        };
        let static_abilities = view
            .calculated_characteristics(obj_id)
            .map(|chars| chars.static_abilities)
            .unwrap_or_else(|| {
                obj.abilities
                    .iter()
                    .filter_map(|ability| {
                        let AbilityKind::Static(static_ability) = &ability.kind else {
                            return None;
                        };
                        Some(static_ability.clone())
                    })
                    .collect::<Vec<_>>()
            });
        static_abilities
            .iter()
            .any(|static_ability| static_ability.id() == ability_id)
    })
}

fn event_has_creature_entering_battlefield(game: &GameState, trigger_event: &TriggerEvent) -> bool {
    let Some(zone_change) = trigger_event.downcast::<crate::events::zones::ZoneChangeEvent>()
    else {
        return false;
    };
    if !zone_change.is_etb() {
        return false;
    }

    zone_change.objects.iter().any(|object_id| {
        game.object(*object_id)
            .is_some_and(|obj| game.object_has_card_type(obj.id, CardType::Creature))
            || zone_change.snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.object_id == *object_id
                    && snapshot.card_types.contains(&CardType::Creature)
            })
    })
}

fn suppresses_creature_etb_triggers(game: &GameState, trigger_event: &TriggerEvent) -> bool {
    suppresses_creature_etb_triggers_with_effects(game, trigger_event, None)
}

fn suppresses_creature_etb_triggers_with_effects(
    game: &GameState,
    trigger_event: &TriggerEvent,
    all_effects: Option<&[ContinuousEffect]>,
) -> bool {
    if !event_has_creature_entering_battlefield(game, trigger_event) {
        return false;
    }

    if let Some(effects) = all_effects {
        return battlefield_has_static_ability_with_effects(
            game,
            StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger,
            effects,
        );
    }

    let effects = game.all_continuous_effects();
    battlefield_has_static_ability_with_effects(
        game,
        StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger,
        &effects,
    )
}

fn trigger_source_matches_duplication_filter(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    entry: &TriggeredAbilityEntry,
    controller: PlayerId,
    static_source: ObjectId,
    filter: &ObjectFilter,
) -> bool {
    let ctx = game.filter_context_for(controller, Some(static_source));

    if let Some(snapshot) = entry.source_snapshot.as_ref() {
        return filter.matches_snapshot(snapshot, &ctx, game);
    }

    let Some(source_obj) = game.object(entry.source) else {
        return false;
    };
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
        source_obj,
        game,
        view.effects(),
    );
    filter.matches_snapshot(&snapshot, &ctx, game)
}

fn trigger_event_matches_duplication_matcher(
    game: &GameState,
    entry: &TriggeredAbilityEntry,
    controller: PlayerId,
    static_source: ObjectId,
    matcher: &Trigger,
) -> bool {
    let ctx = TriggerContext::for_source(static_source, controller, game);
    matcher.matches(&entry.triggering_event, &ctx)
}

fn trigger_entry_matches_specs(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    entry: &TriggeredAbilityEntry,
    controller: PlayerId,
    static_source: ObjectId,
    source_filter: Option<&ObjectFilter>,
    event_matcher: Option<&Trigger>,
) -> bool {
    if let Some(filter) = source_filter
        && !trigger_source_matches_duplication_filter(
            game,
            view,
            entry,
            controller,
            static_source,
            filter,
        )
    {
        return false;
    }
    if let Some(matcher) = event_matcher
        && !trigger_event_matches_duplication_matcher(
            game,
            entry,
            controller,
            static_source,
            matcher,
        )
    {
        return false;
    }
    true
}

fn additional_trigger_copies_for_entry(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    entry: &TriggeredAbilityEntry,
) -> usize {
    let mut copies = 0usize;

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let Some(static_abilities) = view.static_abilities_rc(obj_id) else {
            continue;
        };

        for static_ability in static_abilities.iter() {
            let Some(spec) = static_ability.trigger_duplication_spec() else {
                continue;
            };
            if !trigger_entry_matches_specs(
                game,
                view,
                entry,
                obj.controller,
                obj_id,
                spec.source_filter.as_ref(),
                spec.event_matcher.as_ref(),
            ) {
                continue;
            }
            copies += spec.copies;
        }
    }

    copies
}

fn trigger_is_suppressed(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    entry: &TriggeredAbilityEntry,
) -> bool {
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let Some(static_abilities) = view.static_abilities_rc(obj_id) else {
            continue;
        };

        for static_ability in static_abilities.iter() {
            let Some(spec) = static_ability.trigger_suppression_spec() else {
                continue;
            };
            if trigger_entry_matches_specs(
                game,
                view,
                entry,
                obj.controller,
                obj_id,
                spec.source_filter.as_ref(),
                spec.event_matcher.as_ref(),
            ) {
                return true;
            }
        }
    }

    false
}

fn remove_suppressed_triggers(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    triggered.retain(|entry| !trigger_is_suppressed(game, view, entry));
}

fn append_additional_trigger_copies(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let base_entries = triggered.clone();
    for entry in &base_entries {
        let copies = additional_trigger_copies_for_entry(game, view, entry);
        for _ in 0..copies {
            triggered.push(entry.clone());
        }
    }
}

fn monarch_designation_source() -> (ObjectId, StableId, String) {
    let source = ObjectId::from_raw(0);
    (source, StableId::from(source), "The Monarch".to_string())
}

fn initiative_designation_source() -> (ObjectId, StableId, String) {
    let source = ObjectId::from_raw(u64::MAX - 1);
    (source, StableId::from(source), "The Initiative".to_string())
}

fn ring_designation_source() -> (ObjectId, StableId, String) {
    let source = ObjectId::from_raw(u64::MAX);
    (source, StableId::from(source), "The Ring".to_string())
}

fn push_monarch_trigger(
    triggered: &mut Vec<TriggeredAbilityEntry>,
    controller: PlayerId,
    ability: TriggeredAbility,
    trigger_event: &TriggerEvent,
) {
    let (source, source_stable_id, source_name) = monarch_designation_source();
    let trigger_identity = compute_trigger_identity(&ability);
    triggered.push(TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        trigger_identity,
    });
}

fn push_ring_trigger(
    triggered: &mut Vec<TriggeredAbilityEntry>,
    controller: PlayerId,
    ability: TriggeredAbility,
    trigger_event: &TriggerEvent,
) {
    let (source, source_stable_id, source_name) = ring_designation_source();
    let trigger_identity = compute_trigger_identity(&ability);
    triggered.push(TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        trigger_identity,
    });
}

fn push_initiative_trigger(
    triggered: &mut Vec<TriggeredAbilityEntry>,
    controller: PlayerId,
    ability: TriggeredAbility,
    trigger_event: &TriggerEvent,
) {
    let (source, source_stable_id, source_name) = initiative_designation_source();
    let trigger_identity = compute_trigger_identity(&ability);
    triggered.push(TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        trigger_identity,
    });
}

fn add_monarch_designation_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let Some(monarch) = game.monarch else {
        return;
    };

    if trigger_event.kind() == crate::events::traits::EventKind::BeginningOfEndStep
        && let Some(end_step) =
            trigger_event.downcast::<crate::events::phase::BeginningOfEndStepEvent>()
        && end_step.player == monarch
    {
        push_monarch_trigger(
            triggered,
            monarch,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "monarch_end_step",
                    "At the beginning of the monarch's end step".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![Effect::target_draws(
                    1,
                    PlayerFilter::Specific(monarch),
                )]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && let crate::game_event::DamageTarget::Player(player_id) = damage_event.target
        && player_id == monarch
        && let Some(source_obj) = game.object(damage_event.source)
        && game.object_has_card_type(source_obj.id, CardType::Creature)
    {
        push_monarch_trigger(
            triggered,
            monarch,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "monarch_combat_damage",
                    "Whenever a creature deals combat damage to the monarch".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![Effect::become_monarch_player(
                    PlayerFilter::Specific(source_obj.controller),
                )]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }
}

fn initiative_already_transferred_this_batch(
    game: &GameState,
    damaged_player: PlayerId,
    controller: PlayerId,
) -> bool {
    game.combat_damage_player_batch_hits()
        .iter()
        .filter(|(_, player)| *player == damaged_player)
        .filter_map(|(source, _)| game.object(*source))
        .any(|object| object.controller == controller)
}

fn add_initiative_designation_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let Some(initiative) = game.initiative else {
        return;
    };

    if trigger_event.kind() == crate::events::traits::EventKind::BeginningOfUpkeep
        && let Some(upkeep) =
            trigger_event.downcast::<crate::events::phase::BeginningOfUpkeepEvent>()
        && upkeep.player == initiative
    {
        push_initiative_trigger(
            triggered,
            initiative,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "initiative_upkeep",
                    "At the beginning of the upkeep of the player who has the initiative"
                        .to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![
                    Effect::venture_into_undercity_player(PlayerFilter::Specific(initiative)),
                ]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && let crate::game_event::DamageTarget::Player(player_id) = damage_event.target
        && player_id == initiative
        && let Some(source_obj) = game.object(damage_event.source)
        && game.object_has_card_type(source_obj.id, CardType::Creature)
        && !initiative_already_transferred_this_batch(game, initiative, source_obj.controller)
    {
        push_initiative_trigger(
            triggered,
            initiative,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "initiative_combat_damage",
                    "Whenever one or more creatures a player controls deal combat damage to the player who has the initiative"
                        .to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![Effect::take_initiative_player(
                    PlayerFilter::Specific(source_obj.controller),
                )]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }
}

fn add_ring_designation_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    if trigger_event.kind() == crate::events::traits::EventKind::CreatureAttacked
        && let Some(attacked) =
            trigger_event.downcast::<crate::events::combat::CreatureAttackedEvent>()
        && let Some(attacker) = game.object(attacked.attacker)
        && game.ring_level(attacker.controller) >= 2
        && game.current_ring_bearer(attacker.controller) == Some(attacked.attacker)
    {
        push_ring_trigger(
            triggered,
            attacker.controller,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "ring_bearer_attacks",
                    "Whenever your Ring-bearer attacks".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![
                    Effect::target_draws(1, PlayerFilter::Specific(attacker.controller)),
                    Effect::discard_player(1, PlayerFilter::Specific(attacker.controller), false),
                ]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::CreatureBlocked
        && let Some(blocked) =
            trigger_event.downcast::<crate::events::combat::CreatureBlockedEvent>()
        && let Some(attacker) = game.object(blocked.attacker)
        && game.ring_level(attacker.controller) >= 3
        && game.current_ring_bearer(attacker.controller) == Some(blocked.attacker)
    {
        let delayed = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
            Trigger::end_of_combat(),
            vec![Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::SpecificObject(blocked.blocker),
            ))],
            true,
            vec![blocked.blocker],
            PlayerFilter::Specific(attacker.controller),
        ));
        push_ring_trigger(
            triggered,
            attacker.controller,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "ring_bearer_becomes_blocked",
                    "Whenever your Ring-bearer becomes blocked by a creature".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![delayed]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && matches!(
            damage_event.target,
            crate::game_event::DamageTarget::Player(_)
        )
        && let Some(source_obj) = game.object(damage_event.source)
        && game.ring_level(source_obj.controller) >= 4
        && game.current_ring_bearer(source_obj.controller) == Some(damage_event.source)
    {
        push_ring_trigger(
            triggered,
            source_obj.controller,
            TriggeredAbility {
                trigger: Trigger::custom(
                    "ring_bearer_combat_damage",
                    "Whenever your Ring-bearer deals combat damage to a player".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![Effect::for_each_opponent(vec![
                    Effect::lose_life_player(3, PlayerFilter::IteratedPlayer),
                ])]),
                choices: vec![],
                intervening_if: None,
            },
            trigger_event,
        );
    }
}

/// Check all permanents for triggered abilities that match the given event.
///
/// Returns a list of triggered abilities that should go on the stack.
pub fn check_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
) -> Vec<TriggeredAbilityEntry> {
    let view = crate::derived_view::DerivedGameView::new(game);
    check_triggers_with_view(game, trigger_event, &view)
}

fn for_each_public_nonbattlefield_trigger_object_id(
    game: &GameState,
    mut visit: impl FnMut(ObjectId),
) {
    for player in &game.players {
        for &obj_id in &player.graveyard {
            visit(obj_id);
        }
    }
    for &obj_id in &game.exile {
        visit(obj_id);
    }
    for &obj_id in &game.command_zone {
        visit(obj_id);
    }
    for entry in &game.stack {
        if game
            .object(entry.object_id)
            .is_some_and(|obj| obj.zone == Zone::Stack)
        {
            visit(entry.object_id);
        }
    }
}

fn for_each_hidden_trigger_object_id(game: &GameState, mut visit: impl FnMut(ObjectId)) {
    for player in &game.players {
        for &obj_id in &player.hand {
            visit(obj_id);
        }
    }
}

fn tagged_objects_for_trigger_event(
    game: &GameState,
    trigger_event: &TriggerEvent,
) -> HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>> {
    let mut tagged = HashMap::new();
    if let Some(revealed) = trigger_event.downcast::<crate::events::CardRevealedEvent>()
        && let Some(snapshot) = revealed.snapshot.clone()
    {
        tagged.insert(
            crate::tag::TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
            vec![snapshot],
        );
    }
    if let Some(attacked) = trigger_event.downcast::<crate::events::combat::CreatureAttackedEvent>()
        && attacked.total_attackers >= 2
    {
        let other_attackers: Vec<_> = game
            .combat
            .as_ref()
            .into_iter()
            .flat_map(|combat| combat.attackers.iter())
            .filter(|info| info.creature != attacked.attacker)
            .filter_map(|info| {
                game.object(info.creature)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect();
        if !other_attackers.is_empty() {
            tagged.insert(crate::tag::TagKey::from("other_attacker"), other_attackers);
        }
    }
    tagged
}

pub(crate) fn check_triggers_with_view(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<TriggeredAbilityEntry> {
    if suppresses_creature_etb_triggers_with_effects(game, trigger_event, Some(view.effects())) {
        return Vec::new();
    }

    let mut triggered = Vec::new();

    // Check all permanents on the battlefield
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };

        let ctx = TriggerContext::for_source(obj_id, obj.controller, game);

        // Get calculated abilities (after continuous effects like Humility, Blood Moon)
        let calculated_abilities = view
            .abilities_rc(obj_id)
            .unwrap_or_else(|| Rc::new(obj.abilities.clone()));

        // Check each ability on the permanent
        for ability in calculated_abilities.iter() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };

            if !ability.functions_in(&obj.zone) {
                continue;
            }

            if trigger_ability.trigger.matches(trigger_event, &ctx) {
                let trigger_count = trigger_ability.trigger.trigger_count(trigger_event);
                if trigger_count == 0 {
                    continue;
                }
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

                let entry = TriggeredAbilityEntry {
                    source: obj_id,
                    controller: obj.controller,
                    x_value: trigger_entry_x_value(trigger_event, obj.x_value),
                    ability: TriggeredAbility {
                        trigger: trigger_ability.trigger.clone(),
                        effects: trigger_ability.effects.clone(),
                        choices: trigger_ability.choices.clone(),
                        intervening_if: trigger_ability.intervening_if.clone(),
                    },
                    triggering_event: trigger_event.clone(),
                    source_stable_id: obj.stable_id,
                    source_name: obj.name.clone(),
                    source_snapshot: None,
                    tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                    trigger_identity,
                };
                for _ in 0..trigger_count {
                    triggered.push(entry.clone());
                }
            }
        }
    }

    // Special-case: for leave-the-battlefield zone changes, also allow triggers from
    // the object that left using its last-known information (LKI). This enables
    // triggers like "When this leaves the battlefield" on sources that are no
    // longer on the battlefield when checked.
    if trigger_event.kind() == crate::events::traits::EventKind::ZoneChange
        && let Some(zc) = trigger_event.downcast::<crate::events::zones::ZoneChangeEvent>()
        && zc.is_ltb()
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
                    let trigger_count = trigger_ability.trigger.trigger_count(trigger_event);
                    if trigger_count == 0 {
                        continue;
                    }
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

                    let entry = TriggeredAbilityEntry {
                        source: snapshot.object_id,
                        controller: snapshot.controller,
                        x_value: trigger_entry_x_value(trigger_event, snapshot.x_value),
                        ability: TriggeredAbility {
                            trigger: trigger_ability.trigger.clone(),
                            effects: trigger_ability.effects.clone(),
                            choices: trigger_ability.choices.clone(),
                            intervening_if: trigger_ability.intervening_if.clone(),
                        },
                        triggering_event: trigger_event.clone(),
                        source_stable_id: snapshot.stable_id,
                        source_name: snapshot.name.clone(),
                        source_snapshot: Some(snapshot.clone()),
                        tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                        trigger_identity,
                    };
                    for _ in 0..trigger_count {
                        triggered.push(entry.clone());
                    }
                }
            }
        }
    }

    // Check objects in all public non-battlefield zones.
    for_each_public_nonbattlefield_trigger_object_id(game, |obj_id| {
        check_triggers_in_zone(game, obj_id, trigger_event, &mut triggered);
    });

    // Hand is hidden, but some mechanics (for example Miracle) legitimately trigger there.
    for_each_hidden_trigger_object_id(game, |obj_id| {
        check_triggers_in_zone(game, obj_id, trigger_event, &mut triggered);
    });

    // Note: Undying/Persist/Miracle triggers are handled through the normal trigger system.
    // They function from the graveyard/hand (where the object is after the event) and use
    // the triggering_event to get stable_id and other context at execution time.

    // Cascade: When a spell with cascade is cast, it triggers once for each cascade instance.
    // We model this as a synthetic trigger on SpellCast so it goes on the stack normally.
    if trigger_event.kind() == crate::events::traits::EventKind::SpellCast
        && let Some(cast) = trigger_event.downcast::<crate::events::spells::SpellCastEvent>()
        && let Some(entry) = game.stack.iter().find(|e| e.object_id == cast.spell)
        && let Some(obj) = game.object(cast.spell)
    {
        let native_cascade_count = obj
            .abilities
            .iter()
            .filter(|ability| {
                if !ability.functions_in(&Zone::Stack) {
                    return false;
                }
                let AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                if static_ability.id() == crate::static_abilities::StaticAbilityId::Cascade {
                    return true;
                }
                if let Some(spec) = static_ability.conditional_spell_keyword_spec()
                    && spec.keyword == crate::static_abilities::ConditionalSpellKeywordKind::Cascade
                {
                    return crate::static_abilities::conditional_spell_keyword_active(
                        spec,
                        game,
                        cast.caster,
                    );
                }
                false
            })
            .count();
        let granted_cascade_count = game
            .temporary_granted_spell_abilities(cast.spell, cast.caster)
            .into_iter()
            .filter(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Cascade)
            .count();
        let cascade_count = native_cascade_count + granted_cascade_count;
        if cascade_count > 0 {
            let ability = TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::CascadeEffect::new(),
                )]),
                choices: vec![],
                intervening_if: None,
            };
            let trigger_identity = compute_trigger_identity(&ability);

            for _ in 0..cascade_count {
                triggered.push(TriggeredAbilityEntry {
                    source: cast.spell,
                    controller: cast.caster,
                    x_value: entry.x_value,
                    ability: ability.clone(),
                    triggering_event: trigger_event.clone(),
                    source_stable_id: obj.stable_id,
                    source_name: obj.name.clone(),
                    source_snapshot: None,
                    tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                    trigger_identity,
                });
            }
        }
    }

    // Replicate: When a spell with Replicate is cast, it triggers to copy itself for each time
    // its Replicate cost was paid. (We model this as a synthetic triggered ability so it
    // stacks and can be responded to like the real mechanic.)
    if trigger_event.kind() == crate::events::traits::EventKind::SpellCast
        && let Some(cast) = trigger_event.downcast::<crate::events::spells::SpellCastEvent>()
        && let Some(entry) = game.stack.iter().find(|e| e.object_id == cast.spell)
    {
        let times = entry.optional_costs_paid.times_paid_label("Replicate");
        if times > 0
            && let Some(obj) = game.object(cast.spell)
        {
            let copy_effect_id = crate::effect::EffectId(0);
            let effects = vec![
                Effect::with_id(
                    copy_effect_id.0,
                    Effect::copy_spell_n(crate::target::ChooseSpec::Source, times as i32),
                ),
                Effect::may_choose_new_targets(copy_effect_id),
            ];
            let ability = TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: ResolutionProgram::from_effects(effects),
                choices: vec![],
                intervening_if: None,
            };
            let trigger_identity = compute_trigger_identity(&ability);

            triggered.push(TriggeredAbilityEntry {
                source: cast.spell,
                controller: cast.caster,
                x_value: entry.x_value,
                ability,
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.clone(),
                source_snapshot: None,
                tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                trigger_identity,
            });
        }
    }

    add_monarch_designation_triggers(game, trigger_event, &mut triggered);
    add_initiative_designation_triggers(game, trigger_event, &mut triggered);
    add_ring_designation_triggers(game, trigger_event, &mut triggered);
    remove_suppressed_triggers(game, view, &mut triggered);
    append_additional_trigger_copies(game, view, &mut triggered);

    triggered
}

fn state_trigger_event(source: ObjectId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        crate::events::StateTriggerEvent::new(source),
        crate::provenance::ProvNodeId::default(),
    )
}

fn collect_state_triggers_for_object(
    game: &GameState,
    obj: &crate::object::Object,
    abilities: &[crate::ability::Ability],
    triggered: &mut Vec<TriggeredAbilityEntry>,
    active: &mut HashSet<ActiveStateTriggerKey>,
) {
    for ability in abilities {
        let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
            continue;
        };
        if !ability.functions_in(&obj.zone) {
            continue;
        }
        if trigger_ability
            .trigger
            .downcast_ref::<crate::triggers::StateTrigger>()
            .is_none()
        {
            continue;
        }
        let Some(condition) = trigger_ability.intervening_if.as_ref() else {
            continue;
        };

        let trigger_identity = compute_trigger_identity(trigger_ability);
        let key = ActiveStateTriggerKey {
            source_stable_id: obj.stable_id,
            trigger_identity,
        };
        let trigger_event = state_trigger_event(obj.id);
        if !verify_intervening_if(
            game,
            condition,
            obj.controller,
            &trigger_event,
            obj.id,
            Some(trigger_identity),
        ) {
            continue;
        }

        active.insert(key);
        if game.active_state_trigger_conditions.contains(&key) {
            continue;
        }

        let tagged_objects = tagged_objects_for_trigger_event(game, &trigger_event);
        triggered.push(TriggeredAbilityEntry {
            source: obj.id,
            controller: obj.controller,
            x_value: trigger_entry_x_value(&trigger_event, obj.x_value),
            ability: TriggeredAbility {
                trigger: trigger_ability.trigger.clone(),
                effects: trigger_ability.effects.clone(),
                choices: trigger_ability.choices.clone(),
                intervening_if: trigger_ability.intervening_if.clone(),
            },
            triggering_event: trigger_event,
            source_stable_id: obj.stable_id,
            source_name: obj.name.clone(),
            source_snapshot: None,
            tagged_objects,
            trigger_identity,
        });
    }
}

/// Check all current state-triggered abilities and return newly-triggered entries plus
/// the set of state-trigger conditions that are currently true.
pub fn check_state_triggers(
    game: &GameState,
) -> (Vec<TriggeredAbilityEntry>, HashSet<ActiveStateTriggerKey>) {
    let view = crate::derived_view::DerivedGameView::new(game);
    let mut triggered = Vec::new();
    let mut active = HashSet::new();

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_abilities = view
            .abilities_rc(obj_id)
            .unwrap_or_else(|| Rc::new(obj.abilities.clone()));
        collect_state_triggers_for_object(
            game,
            obj,
            calculated_abilities.as_ref(),
            &mut triggered,
            &mut active,
        );
    }

    for_each_public_nonbattlefield_trigger_object_id(game, |obj_id| {
        if let Some(obj) = game.object(obj_id) {
            collect_state_triggers_for_object(
                game,
                obj,
                &obj.abilities,
                &mut triggered,
                &mut active,
            );
        }
    });

    for_each_hidden_trigger_object_id(game, |obj_id| {
        if let Some(obj) = game.object(obj_id) {
            collect_state_triggers_for_object(
                game,
                obj,
                &obj.abilities,
                &mut triggered,
                &mut active,
            );
        }
    });

    (triggered, active)
}

/// Check delayed triggers against an event and return triggered entries.
pub fn check_delayed_triggers(
    game: &mut GameState,
    trigger_event: &TriggerEvent,
) -> Vec<TriggeredAbilityEntry> {
    if suppresses_creature_etb_triggers(game, trigger_event) {
        return Vec::new();
    }

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
        let fallback_source = ObjectId::from_raw(0);
        let candidate_sources: &[ObjectId] = if delayed.target_objects.is_empty() {
            std::slice::from_ref(&fallback_source)
        } else {
            delayed.target_objects.as_slice()
        };
        let trigger_identity = compute_delayed_trigger_identity(delayed);

        let mut fired = false;
        for &source in candidate_sources {
            let ctx = TriggerContext::for_source(source, delayed.controller, game);
            if !delayed.trigger.matches(trigger_event, &ctx) {
                continue;
            }

            fired = true;
            let ability_source = delayed.ability_source.unwrap_or(source);
            let source_stable_id = delayed
                .ability_source_stable_id
                .or_else(|| game.object(ability_source).map(|o| o.stable_id))
                .or_else(|| {
                    delayed
                        .ability_source_stable_id
                        .and_then(|stable_id| game.find_object_by_stable_id(stable_id))
                        .and_then(|id| game.object(id))
                        .map(|o| o.stable_id)
                })
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
            let source_name = delayed
                .ability_source_name
                .clone()
                .or_else(|| game.object(ability_source).map(|o| o.name.clone()))
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
                x_value: delayed.x_value,
                ability: TriggeredAbility {
                    trigger: delayed.trigger.clone(),
                    effects: delayed.effects.clone(),
                    choices: delayed.choices.clone(),
                    intervening_if: None,
                },
                triggering_event: trigger_event.clone(),
                source_stable_id,
                source_name,
                source_snapshot: delayed.ability_source_snapshot.clone(),
                tagged_objects: {
                    let mut tagged = delayed.tagged_objects.clone();
                    for (tag, snapshots) in tagged_objects_for_trigger_event(game, trigger_event) {
                        tagged.entry(tag).or_default().extend(snapshots);
                    }
                    tagged
                },
                trigger_identity,
            });

            if delayed.one_shot {
                break;
            }
        }

        if fired && delayed.one_shot {
            to_remove.push(idx);
        }
    }

    if !to_remove.is_empty() {
        to_remove.sort_unstable();
        to_remove.dedup();
        let mut remove_iter = to_remove.into_iter().peekable();
        let mut idx = 0usize;
        game.delayed_triggers.retain(|_| {
            let remove = remove_iter.peek().is_some_and(|next| *next == idx);
            if remove {
                remove_iter.next();
            }
            idx += 1;
            !remove
        });
    }

    let view = crate::derived_view::DerivedGameView::new(game);
    remove_suppressed_triggers(game, &view, &mut triggered);

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

    let ctx = TriggerContext::for_source(obj_id, obj.controller, game);

    for ability in &obj.abilities {
        let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
            continue;
        };

        if !ability.functions_in(&obj.zone) {
            continue;
        }

        if trigger_ability.trigger.matches(trigger_event, &ctx) {
            let trigger_count = trigger_ability.trigger.trigger_count(trigger_event);
            if trigger_count == 0 {
                continue;
            }
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

            let entry = TriggeredAbilityEntry {
                source: obj_id,
                controller: obj.controller,
                x_value: trigger_entry_x_value(trigger_event, obj.x_value),
                ability: TriggeredAbility {
                    trigger: trigger_ability.trigger.clone(),
                    effects: trigger_ability.effects.clone(),
                    choices: trigger_ability.choices.clone(),
                    intervening_if: trigger_ability.intervening_if.clone(),
                },
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.clone(),
                source_snapshot: None,
                tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                trigger_identity,
            };
            for _ in 0..trigger_count {
                triggered.push(entry.clone());
            }
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
        PlayerFilter::MostLifeTied => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.life)
            .max()
            .is_some_and(|max_life| {
                game.player(player)
                    .is_some_and(|candidate| candidate.is_in_game() && candidate.life == max_life)
            }),
        PlayerFilter::MostCardsInHand => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.hand.len())
            .max()
            .and_then(|max_hand| {
                let leaders = game
                    .players
                    .iter()
                    .filter(|candidate| candidate.is_in_game() && candidate.hand.len() == max_hand)
                    .map(|candidate| candidate.id)
                    .collect::<Vec<_>>();
                match leaders.as_slice() {
                    [leader] => Some(*leader == player),
                    _ => None,
                }
            })
            .unwrap_or(false),
        PlayerFilter::CastCardTypeThisTurn(card_type) => game
            .turn_history
            .spell_cast_snapshot_history()
            .iter()
            .any(|snapshot| {
                snapshot.controller == player && snapshot.card_types.contains(card_type)
            }),
        PlayerFilter::ChosenPlayer => false,
        PlayerFilter::TaggedPlayer(_) => false,
        PlayerFilter::Teammate => false,
        PlayerFilter::Attacking => false,
        PlayerFilter::DamagedPlayer => false,
        PlayerFilter::EffectController => player == controller,
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
        PlayerFilter::AliasedControllerOf(obj_ref) => match obj_ref {
            ObjectRef::Specific(object_id) => game
                .object(*object_id)
                .is_some_and(|obj| player == obj.controller),
            ObjectRef::Target | ObjectRef::Tagged(_) => false,
        },
        PlayerFilter::AliasedOwnerOf(obj_ref) => match obj_ref {
            ObjectRef::Specific(object_id) => game
                .object(*object_id)
                .is_some_and(|obj| player == obj.owner),
            ObjectRef::Target | ObjectRef::Tagged(_) => false,
        },
        PlayerFilter::Active => player == game.turn.active_player,
        PlayerFilter::Defending => defending_player == Some(player),
        PlayerFilter::IteratedPlayer => false,
        PlayerFilter::TargetPlayerOrControllerOfTarget => false,
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_matches_with_context(base, player, controller, game, defending_player)
                && !player_filter_matches_with_context(
                    excluded,
                    player,
                    controller,
                    game,
                    defending_player,
                )
        }
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
        (Phase::Beginning, Some(Step::Upkeep)) => Some(TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::Beginning, Some(Step::Draw)) => Some(TriggerEvent::new_with_provenance(
            BeginningOfDrawStepEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::FirstMain, None) => Some(TriggerEvent::new_with_provenance(
            BeginningOfPrecombatMainPhaseEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::Combat, Some(Step::BeginCombat)) => Some(TriggerEvent::new_with_provenance(
            BeginningOfCombatEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::Combat, Some(Step::EndCombat)) => Some(TriggerEvent::new_with_provenance(
            EndOfCombatEvent::new(),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::NextMain, None) => Some(TriggerEvent::new_with_provenance(
            BeginningOfPostcombatMainPhaseEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        (Phase::Ending, Some(Step::End)) => Some(TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(active),
            crate::provenance::ProvNodeId::default(),
        )),
        _ => None,
    }
}

/// Verify if an intervening-if condition is met.
pub fn verify_intervening_if(
    game: &GameState,
    condition: &crate::ConditionExpr,
    controller: PlayerId,
    event: &TriggerEvent,
    source_object_id: ObjectId,
    trigger_identity: Option<TriggerIdentity>,
) -> bool {
    let defending_player = if event.kind() == crate::events::traits::EventKind::CreatureAttacked {
        event.downcast::<crate::events::combat::CreatureAttackedEvent>()
            .and_then(|attacked| match attacked.target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => {
                    game.object(planeswalker_id).map(|planeswalker| planeswalker.controller)
                }
            })
    } else if event.kind() == crate::events::traits::EventKind::CreatureAttackedAndUnblocked {
        event.downcast::<crate::events::combat::CreatureAttackedAndUnblockedEvent>()
            .and_then(|attacked| match attacked.target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => {
                    game.object(planeswalker_id).map(|planeswalker| planeswalker.controller)
                }
            })
    } else if event.kind() == crate::events::traits::EventKind::CreatureBecameBlocked {
        event.downcast::<crate::events::combat::CreatureBecameBlockedEvent>()
            .and_then(|blocked| blocked.attack_target)
            .and_then(|target| match target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => {
                    game.object(planeswalker_id).map(|planeswalker| planeswalker.controller)
                }
            })
    } else {
        None
    };
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source: source_object_id,
        defending_player,
        attacking_player: None,
        // Legacy intervening-if checks intentionally did not provide a filter-context source.
        filter_source: None,
        triggering_event: Some(event),
        trigger_identity,
        ability_index: None,
        options: Default::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::combat_state::AttackTarget;
    use crate::events::DamageEvent;
    use crate::events::cause::EventCause;
    use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent, CreatureBlockedEvent};
    use crate::events::other::BecameMonstrousEvent;
    use crate::events::spells::SpellCastEvent;
    use crate::game_event::DamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn make_battlefield_creature(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn temporary_next_spell_cascade_grant_triggers_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let spell = CardBuilder::new(CardId::from_raw(9001), "Test Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, alice));

        game.add_temporary_spell_ability_grant(
            alice,
            spell_id,
            crate::target::ObjectFilter::noncreature_spell().cast_by(crate::PlayerFilter::You),
            StaticAbility::cascade(),
            1,
        );

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell_id, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert_eq!(triggered.len(), 1, "expected one cascade trigger");

        game.consume_temporary_spell_ability_grants_for_spell(spell_id, alice);
        assert!(
            game.temporary_granted_spell_abilities(spell_id, alice)
                .is_empty(),
            "grant should be consumed after the cast event resolves"
        );
    }

    #[test]
    fn conspire_paid_spell_cast_creates_one_trigger_per_paid_instance() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let def = CardDefinitionBuilder::new(CardId::from_raw(9002), "Conspire Test Spell")
            .card_types(vec![CardType::Sorcery])
            .conspire()
            .conspire()
            .with_spell_effect(vec![crate::effect::Effect::draw(1)])
            .build();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
        let mut entry = crate::game_state::StackEntry::new(spell_id, alice);
        entry.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
        entry.optional_costs_paid.pay(0);
        entry.optional_costs_paid.pay(1);
        game.push_to_stack(entry);
        game.object_mut(spell_id)
            .expect("spell object should exist")
            .optional_costs_paid = crate::cost::OptionalCostsPaid {
            costs: vec![("Conspire".to_string(), 1), ("Conspire 2".to_string(), 1)],
        };

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell_id, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(
            triggered.len(),
            2,
            "expected two separate conspire triggers"
        );
        for trigger in &triggered {
            let debug = format!("{:?}", trigger.ability.effects);
            assert!(
                debug.contains("CopySpellEffect"),
                "expected each conspire trigger to copy the spell, got {debug}"
            );
        }
    }

    #[test]
    fn ring_designation_attack_trigger_draws_then_discards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bearer = make_battlefield_creature(&mut game, alice, "Bearer");

        game.increment_ring_temptations(alice);
        game.increment_ring_temptations(alice);
        game.set_ring_bearer(alice, bearer);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::new(bearer, AttackEventTarget::Player(bob)),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source_name, "The Ring");
        let effects = triggered[0].ability.effects.all_effects();
        assert!(effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some()
        }));
        assert!(effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::DiscardEffect>()
                .is_some()
        }));
    }

    #[test]
    fn creature_attacked_event_captures_other_attackers_tag() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_creature(&mut game, alice, "Creepy Puppeteer");
        let partner = make_battlefield_creature(&mut game, alice, "Backup Attacker");

        let combat = game.combat.get_or_insert_with(Default::default);
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: partner,
            target: AttackTarget::Player(bob),
        });

        let trigger_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(source, AttackEventTarget::Player(bob), 2),
            crate::provenance::ProvNodeId::default(),
        );
        let tagged = tagged_objects_for_trigger_event(&game, &trigger_event);
        let other_attackers = tagged
            .get(&crate::tag::TagKey::from("other_attacker"))
            .expect("expected other_attacker tag for exact partner attack event");

        assert_eq!(other_attackers.len(), 1);
        assert_eq!(other_attackers[0].object_id, partner);
    }

    #[test]
    fn ring_designation_block_trigger_schedules_end_of_combat_sacrifice() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bearer = make_battlefield_creature(&mut game, alice, "Bearer");
        let blocker = make_battlefield_creature(&mut game, bob, "Blocker");

        for _ in 0..3 {
            game.increment_ring_temptations(alice);
        }
        game.set_ring_bearer(alice, bearer);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                CreatureBlockedEvent::new(blocker, bearer),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1);
        let schedule = triggered[0]
            .ability
            .effects
            .all_effects()
            .iter()
            .find_map(|effect| {
                effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            })
            .expect("expected delayed end-of-combat sacrifice");
        assert!(schedule.trigger.display().contains("end of combat"));
        let sacrifice = schedule
            .effects
            .all_effects()
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::SacrificeTargetEffect>())
            .expect("expected sacrifice effect");
        assert_eq!(sacrifice.target, ChooseSpec::SpecificObject(blocker));
    }

    #[test]
    fn ring_designation_combat_damage_trigger_hits_each_opponent() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bearer = make_battlefield_creature(&mut game, alice, "Bearer");

        for _ in 0..4 {
            game.increment_ring_temptations(alice);
        }
        game.set_ring_bearer(alice, bearer);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                DamageEvent::with_cause(
                    bearer,
                    DamageTarget::Player(bob),
                    2,
                    true,
                    EventCause::combat_damage(bearer),
                ),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1);
        assert!(
            triggered[0]
                .ability
                .effects
                .all_effects()
                .iter()
                .any(|effect| effect
                    .downcast_ref::<crate::effects::ForPlayersEffect>()
                    .is_some())
        );
    }

    #[test]
    fn became_monstrous_trigger_uses_event_n_as_x_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let def = CardDefinitionBuilder::new(CardId::new(), "Vitality Hunter")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 4))
            .parse_text(
                "Lifelink\n{X}{W}{W}: Monstrosity X. (If this creature isn't monstrous, put X +1/+1 counters on it and it becomes monstrous.)\nWhen this creature becomes monstrous, put a lifelink counter on each of up to X target creatures.",
            )
            .expect("parse Vitality Hunter text");
        let hunter_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                BecameMonstrousEvent::new(hunter_id, alice, 4),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1, "expected one becomes-monstrous trigger");
        assert_eq!(
            triggered[0].x_value,
            Some(4),
            "trigger should remember the monstrosity value as X"
        );
        assert!(
            matches!(
                triggered[0].ability.choices.first(),
                Some(ChooseSpec::WithCount(_, count)) if count.is_up_to_dynamic_x()
            ),
            "expected up-to-X target choice on Vitality Hunter trigger, got {:?}",
            triggered[0].ability.choices
        );
    }
}
