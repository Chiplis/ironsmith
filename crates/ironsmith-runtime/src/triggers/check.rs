//! Trigger checking and queue management.
//!
//! This module contains the `check_triggers()` function that scans all permanents
//! for triggered abilities that match a game event.

use crate::filter::ObjectFilterExt as _;
use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::Effect;
use crate::FxMap;
use crate::ability::{AbilityKind, PresentationLabel, TriggeredAbility};
use crate::continuous::ContinuousEffect;
use crate::effect::Value;
use crate::events::EventKind;
use crate::filter::ObjectRef;
use crate::filter::{Comparison, ObjectFilter};
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::resolution::ResolutionProgram;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::{StaticAbilityId, TriggerDuplicationSourceMatcher};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::Trigger;
use super::TriggerEvent;
use super::matcher_trait::TriggerContext;

const SPEED_RULE_SOURCE_NAME: &str = "Start your engines";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerRegistryKey {
    effects_revision: u64,
    mutation_revision: u64,
    zone_revision: u64,
    continuous_context_revision: u64,
    turn_number: u32,
    active_player: PlayerId,
    phase: Phase,
    step: Option<Step>,
    combat_phases_started_this_turn: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct TriggerRegistry {
    pub(crate) key: TriggerRegistryKey,
    by_kind: FxMap<EventKind, Vec<TriggerSubscriber>>,
    source_local_by_kind: FxMap<EventKind, FxMap<ObjectId, Vec<TriggerSubscriber>>>,
    wildcard: Vec<TriggerSubscriber>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TriggerSubscriber {
    ordinal: u32,
    source: ObjectId,
    ability_index: usize,
}

impl TriggerRegistry {
    fn subscribers_for(
        &self,
        kind: EventKind,
        event_object: Option<ObjectId>,
    ) -> Vec<TriggerSubscriber> {
        let mut subscribers = self.wildcard.clone();
        if let Some(kind_subscribers) = self.by_kind.get(&kind) {
            subscribers.extend(kind_subscribers.iter().copied());
        }
        if let Some(event_object) = event_object
            && let Some(source_subscribers) = self
                .source_local_by_kind
                .get(&kind)
                .and_then(|by_source| by_source.get(&event_object))
        {
            subscribers.extend(source_subscribers.iter().copied());
        }
        subscribers.sort_by_key(|subscriber| subscriber.ordinal);
        subscribers.dedup_by_key(|subscriber| subscriber.ordinal);
        subscribers
    }
}

pub(crate) fn speed_rule_source_id() -> ObjectId {
    ObjectId::from_raw(0)
}

pub(crate) fn is_speed_rule_trigger(entry: &TriggeredAbilityEntry) -> bool {
    entry.source == speed_rule_source_id() && entry.source_name == SPEED_RULE_SOURCE_NAME
}

fn trigger_entry_x_value(trigger_event: &TriggerEvent, fallback: Option<u32>) -> Option<u32> {
    trigger_event
        .downcast::<crate::events::other::BecameMonstrousEvent>()
        .map(|event| event.n)
        .or_else(|| {
            trigger_event
                .downcast::<crate::events::spells::AbilityActivatedEvent>()
                .and_then(|event| event.x_value)
        })
        .or(fallback)
}

fn dynamic_soulshift_value_from_choose_spec(spec: &ChooseSpec) -> Option<&Value> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => dynamic_soulshift_value_from_choose_spec(spec),
        ChooseSpec::Object(filter) => {
            if filter.zone != Some(Zone::Graveyard)
                || filter.owner != Some(PlayerFilter::You)
                || !filter.subtypes.contains(&Subtype::Spirit)
            {
                return None;
            }
            match filter.mana_value.as_ref()? {
                Comparison::LessThanOrEqualExpr(value) => Some(value.as_ref()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn count_filter_from_trigger_lki(
    game: &GameState,
    trigger_event: &TriggerEvent,
    controller: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
) -> i32 {
    let filter_ctx = game.filter_context_for(controller, Some(source));
    let mut seen = HashSet::new();
    let mut count = 0i32;

    for object in game.objects_in_deterministic_order() {
        if filter.matches(object, &filter_ctx, game) {
            seen.insert(object.stable_id);
            count += 1;
        }
    }

    if let Some(zone_change) = trigger_event.downcast::<crate::events::zones::ZoneChangeEvent>() {
        for snapshot in zone_change.snapshots() {
            if !seen.insert(snapshot.stable_id) {
                continue;
            }
            if filter.matches_snapshot(snapshot, &filter_ctx, game) {
                count += 1;
            }
        }
    } else if let Some(snapshot) = trigger_event.snapshot()
        && seen.insert(snapshot.stable_id)
        && filter.matches_snapshot(snapshot, &filter_ctx, game)
    {
        count += 1;
    }

    count
}

fn resolve_dynamic_soulshift_lki_value(
    game: &GameState,
    trigger_event: &TriggerEvent,
    controller: PlayerId,
    source: ObjectId,
    value: &Value,
) -> Option<i32> {
    match value {
        Value::SurfaceHinted { value, .. } => {
            resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, value)
        }
        Value::Fixed(value) => Some(*value),
        Value::Add(left, right) => Some(
            resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, left)?
                + resolve_dynamic_soulshift_lki_value(
                    game,
                    trigger_event,
                    controller,
                    source,
                    right,
                )?,
        ),
        Value::Scaled(value, multiplier) => {
            resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, value)
                .map(|value| value * *multiplier)
        }
        Value::DividedRoundedDown(value, divisor) if *divisor != 0 => {
            resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, value)
                .map(|value| value.div_euclid(*divisor))
        }
        Value::Min(left, right) => Some(
            resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, left)?
                .min(resolve_dynamic_soulshift_lki_value(
                    game,
                    trigger_event,
                    controller,
                    source,
                    right,
                )?),
        ),
        Value::Count(filter) => Some(count_filter_from_trigger_lki(
            game,
            trigger_event,
            controller,
            source,
            filter,
        )),
        Value::CountScaled(filter, multiplier) => Some(
            count_filter_from_trigger_lki(game, trigger_event, controller, source, filter)
                * *multiplier,
        ),
        _ => None,
    }
}

fn captured_dynamic_soulshift_x_value(
    game: &GameState,
    trigger_event: &TriggerEvent,
    controller: PlayerId,
    source: ObjectId,
    trigger_ability: &TriggeredAbility,
) -> Option<u32> {
    let value = trigger_ability
        .choices
        .iter()
        .find_map(dynamic_soulshift_value_from_choose_spec)?;
    let resolved =
        resolve_dynamic_soulshift_lki_value(game, trigger_event, controller, source, value)?;
    u32::try_from(resolved.max(0)).ok()
}

fn freeze_soulshift_object_filter(mut filter: ObjectFilter, x_value: u32) -> ObjectFilter {
    if matches!(filter.mana_value, Some(Comparison::LessThanOrEqualExpr(_))) {
        filter.mana_value = Some(Comparison::LessThanOrEqual(x_value as i32));
    }
    filter
}

fn freeze_soulshift_choose_spec(spec: &ChooseSpec, x_value: u32) -> ChooseSpec {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(freeze_soulshift_choose_spec(spec, x_value)),
            hints: hints.clone(),
        },
        ChooseSpec::Target(spec) => {
            ChooseSpec::Target(Box::new(freeze_soulshift_choose_spec(spec, x_value)))
        }
        ChooseSpec::WithCount(spec, count) => ChooseSpec::WithCount(
            Box::new(freeze_soulshift_choose_spec(spec, x_value)),
            *count,
        ),
        ChooseSpec::WithCountValue(spec, count, value) => ChooseSpec::WithCountValue(
            Box::new(freeze_soulshift_choose_spec(spec, x_value)),
            *count,
            value.clone(),
        ),
        ChooseSpec::Object(filter) => {
            ChooseSpec::Object(freeze_soulshift_object_filter(filter.clone(), x_value))
        }
        _ => spec.clone(),
    }
}

fn freeze_soulshift_effect(effect: Effect, x_value: u32) -> Effect {
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        let mut return_effect = return_effect.clone();
        return_effect.target = freeze_soulshift_choose_spec(&return_effect.target, x_value);
        return Effect::new(return_effect);
    }
    effect
}

fn queued_triggered_ability(
    trigger_ability: &TriggeredAbility,
    dynamic_soulshift_x: Option<u32>,
) -> TriggeredAbility {
    let (effects, choices) = if let Some(x_value) = dynamic_soulshift_x {
        let effects = trigger_ability
            .effects
            .clone()
            .try_map_effects(|effect| Ok::<Effect, ()>(freeze_soulshift_effect(effect, x_value)))
            .expect("freezing soulshift effects should be infallible");
        let choices = trigger_ability
            .choices
            .iter()
            .map(|choice| freeze_soulshift_choose_spec(choice, x_value))
            .collect();
        (effects, choices)
    } else {
        (
            trigger_ability.effects.clone(),
            trigger_ability.choices.clone(),
        )
    };

    TriggeredAbility {
        trigger: trigger_ability.trigger.clone(),
        effects,
        choices,
        intervening_if: trigger_ability.intervening_if.clone(),
        presentation_label: None,
    }
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
    /// Numeric value derived by the trigger from the matched event context.
    pub event_value_amount: Option<i32>,
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
    /// What kind of ability source produced this trigger.
    pub source_kind: TriggeredAbilitySourceKind,
    /// Structural identity of this trigger ability.
    pub trigger_identity: TriggerIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TriggeredAbilitySourceKind {
    #[default]
    Object,
    DungeonRoom,
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

const ATTACHED_SOURCE_TAG: &str = "attached_source";

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
                    .into()
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
    source_matcher: TriggerDuplicationSourceMatcher,
) -> bool {
    match source_matcher {
        TriggerDuplicationSourceMatcher::ObjectAbility => {
            if entry.source_kind != TriggeredAbilitySourceKind::Object {
                return false;
            }
        }
        TriggerDuplicationSourceMatcher::DungeonRoomAbilityOwnedByStaticController => {
            if entry.source_kind != TriggeredAbilitySourceKind::DungeonRoom
                || entry.controller != controller
            {
                return false;
            }
        }
    }
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
                game.controller_of(obj),
                obj_id,
                spec.source_filter.as_ref(),
                spec.event_matcher.as_ref(),
                spec.source_matcher,
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
                game.controller_of(obj),
                obj_id,
                spec.source_filter.as_ref(),
                spec.event_matcher.as_ref(),
                TriggerDuplicationSourceMatcher::ObjectAbility,
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
        event_value_amount: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
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
        event_value_amount: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
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
        event_value_amount: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id,
        source_name,
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
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
                presentation_label: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && let crate::events::DamageTarget::Player(player_id) = damage_event.target
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
                    PlayerFilter::Specific(game.controller_of(source_obj)),
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
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
        .any(|object| game.controller_of(object) == controller)
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
                presentation_label: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && let crate::events::DamageTarget::Player(player_id) = damage_event.target
        && player_id == initiative
        && let Some(source_obj) = game.object(damage_event.source)
        && game.object_has_card_type(source_obj.id, CardType::Creature)
        && !initiative_already_transferred_this_batch(
            game,
            initiative,
            game.controller_of(source_obj),
        )
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
                    PlayerFilter::Specific(game.controller_of(source_obj)),
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
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
        && game.ring_level(game.controller_of(attacker)) >= 2
        && game.current_ring_bearer(game.controller_of(attacker)) == Some(attacked.attacker)
    {
        push_ring_trigger(
            triggered,
            game.controller_of(attacker),
            TriggeredAbility {
                trigger: Trigger::custom(
                    "ring_bearer_attacks",
                    "Whenever your Ring-bearer attacks".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![
                    Effect::target_draws(1, PlayerFilter::Specific(game.controller_of(attacker))),
                    Effect::discard_player(
                        1,
                        PlayerFilter::Specific(game.controller_of(attacker)),
                        false,
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::CreatureBlocked
        && let Some(blocked) =
            trigger_event.downcast::<crate::events::combat::CreatureBlockedEvent>()
        && let Some(attacker) = game.object(blocked.attacker)
        && game.ring_level(game.controller_of(attacker)) >= 3
        && game.current_ring_bearer(game.controller_of(attacker)) == Some(blocked.attacker)
    {
        let delayed = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
            Trigger::end_of_combat(),
            vec![Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::SpecificObject(blocked.blocker),
            ))],
            true,
            vec![blocked.blocker],
            PlayerFilter::Specific(game.controller_of(attacker)),
        ));
        push_ring_trigger(
            triggered,
            game.controller_of(attacker),
            TriggeredAbility {
                trigger: Trigger::custom(
                    "ring_bearer_becomes_blocked",
                    "Whenever your Ring-bearer becomes blocked by a creature".to_string(),
                ),
                effects: ResolutionProgram::from_effects(vec![delayed]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            },
            trigger_event,
        );
    }

    if trigger_event.kind() == crate::events::traits::EventKind::Damage
        && let Some(damage_event) = trigger_event.downcast::<crate::events::damage::DamageEvent>()
        && damage_event.is_combat
        && damage_event.amount > 0
        && matches!(damage_event.target, crate::events::DamageTarget::Player(_))
        && let Some(source_obj) = game.object(damage_event.source)
        && game.ring_level(game.controller_of(source_obj)) >= 4
        && game.current_ring_bearer(game.controller_of(source_obj)) == Some(damage_event.source)
    {
        push_ring_trigger(
            triggered,
            game.controller_of(source_obj),
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
                presentation_label: None,
            },
            trigger_event,
        );
    }
}

fn is_soulbond_pair_trigger(trigger_ability: &TriggeredAbility) -> bool {
    trigger_ability
        .effects
        .all_effects()
        .into_iter()
        .any(|effect| {
            effect
                .downcast_ref::<crate::effects::SoulbondPairEffect>()
                .is_some()
        })
}

fn soulbond_trigger_had_eligible_pair(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> bool {
    let Some(source_obj) = game.object(source) else {
        return false;
    };
    if source_obj.zone != Zone::Battlefield
        || game.controller_of(source_obj) != controller
        || !game.current_is_creature(source)
        || game.is_soulbond_paired(source)
    {
        return false;
    }

    game.battlefield.iter().copied().any(|candidate| {
        candidate != source
            && !game.is_soulbond_paired(candidate)
            && game.object(candidate).is_some_and(|object| {
                object.zone == Zone::Battlefield
                    && game.controller_of(object) == controller
                    && game.current_is_creature(candidate)
            })
    })
}

/// Check all permanents for triggered abilities that match the given event.
///
/// Returns a list of triggered abilities that should go on the stack.
pub fn check_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
) -> Vec<TriggeredAbilityEntry> {
    // LKI payloads are common on zone-change events even when none of the
    // sources represented by those payloads can trigger for this event kind.
    // Inspect the captured ability lists before constructing a layered view;
    // otherwise an unrelated LKI snapshot forces a full battlefield trigger
    // registry rebuild after every simultaneous zone change.
    let lki_may_subscribe = trigger_event
        .lookback_source_snapshots()
        .iter()
        .any(|snapshot| snapshot_may_subscribe_to_event(snapshot, trigger_event.kind()));
    let direct_snapshot_may_subscribe = matches!(
        trigger_event.kind(),
        crate::events::traits::EventKind::Sacrifice
            | crate::events::traits::EventKind::CardDiscarded
    ) && trigger_event
        .snapshot()
        .is_some_and(|snapshot| snapshot_may_subscribe_to_event(snapshot, trigger_event.kind()));

    if !trigger_event_can_have_synthetic_triggers(trigger_event)
        && !game.may_have_triggered_abilities_for_event_kind(trigger_event.kind())
        && !lki_may_subscribe
        && !direct_snapshot_may_subscribe
    {
        return Vec::new();
    }

    let view = crate::derived_view::DerivedGameView::new(game);
    check_triggers_with_view(game, trigger_event, &view)
}

/// Check a group of events that all observe the same stable game state.
///
/// The derived view and battlefield trigger registry are shared across the
/// group. Results retain event order and per-event trigger ordering.
pub(crate) fn check_triggers_batch(
    game: &GameState,
    trigger_events: &[TriggerEvent],
) -> Vec<Vec<TriggeredAbilityEntry>> {
    if trigger_events.is_empty() {
        return Vec::new();
    }

    let mut kind_may_subscribe = FxMap::default();
    let should_check = trigger_events
        .iter()
        .map(|trigger_event| {
            let lki_may_subscribe = trigger_event
                .lookback_source_snapshots()
                .iter()
                .any(|snapshot| snapshot_may_subscribe_to_event(snapshot, trigger_event.kind()));
            let direct_snapshot_may_subscribe = matches!(
                trigger_event.kind(),
                crate::events::traits::EventKind::Sacrifice
                    | crate::events::traits::EventKind::CardDiscarded
            ) && trigger_event.snapshot().is_some_and(
                |snapshot| snapshot_may_subscribe_to_event(snapshot, trigger_event.kind()),
            );
            let current_state_may_subscribe =
                trigger_event_can_have_synthetic_triggers(trigger_event)
                    || *kind_may_subscribe
                        .entry(trigger_event.kind())
                        .or_insert_with(|| {
                            game.may_have_triggered_abilities_for_event_kind(trigger_event.kind())
                        });

            current_state_may_subscribe || lki_may_subscribe || direct_snapshot_may_subscribe
        })
        .collect::<Vec<_>>();

    if !should_check.iter().any(|should_check| *should_check) {
        return vec![Vec::new(); trigger_events.len()];
    }

    let view = crate::derived_view::DerivedGameView::new(game);
    let registry = battlefield_trigger_registry(game, &view);
    trigger_events
        .iter()
        .zip(should_check)
        .map(|(trigger_event, should_check)| {
            if should_check {
                check_triggers_with_view_and_registry(game, trigger_event, &view, &registry)
            } else {
                Vec::new()
            }
        })
        .collect()
}

fn snapshot_may_subscribe_to_event(snapshot: &ObjectSnapshot, event_kind: EventKind) -> bool {
    snapshot.abilities.iter().any(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return false;
        };
        ability.functions_in(&snapshot.zone)
            && triggered
                .trigger
                .subscribed_kinds()
                .is_none_or(|kinds| kinds.contains(&event_kind))
    })
}

fn trigger_event_can_have_synthetic_triggers(trigger_event: &TriggerEvent) -> bool {
    matches!(
        trigger_event.kind(),
        crate::events::traits::EventKind::SpellCast
            | crate::events::traits::EventKind::LifeLoss
            | crate::events::traits::EventKind::Damage
            | crate::events::traits::EventKind::BeginningOfEndStep
            | crate::events::traits::EventKind::BeginningOfUpkeep
            | crate::events::traits::EventKind::CreatureAttacked
            | crate::events::traits::EventKind::CreatureBlocked
    )
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

fn trigger_requires_other_attacker_tag(trigger: &Trigger) -> bool {
    if let Some(with_others) =
        trigger.downcast_ref::<crate::triggers::ThisAttacksWithNOthersTrigger>()
    {
        return with_others.exact && with_others.other_count == 1;
    }
    trigger
        .downcast_ref::<crate::triggers::OrTrigger>()
        .is_some_and(|or_trigger| {
            or_trigger
                .triggers
                .iter()
                .any(trigger_requires_other_attacker_tag)
        })
}

fn tagged_objects_for_matched_trigger(
    game: &GameState,
    trigger_event: &TriggerEvent,
    trigger: &Trigger,
) -> HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>> {
    tagged_objects_for_trigger_event_impl(
        game,
        trigger_event,
        trigger_requires_other_attacker_tag(trigger),
    )
}

fn tagged_objects_for_trigger_event(
    game: &GameState,
    trigger_event: &TriggerEvent,
) -> HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>> {
    tagged_objects_for_trigger_event_impl(game, trigger_event, true)
}

fn tagged_objects_for_trigger_event_impl(
    game: &GameState,
    trigger_event: &TriggerEvent,
    include_other_attackers: bool,
) -> HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>> {
    let mut tagged = HashMap::new();
    if let Some(source) = trigger_event.source_snapshot().cloned().or_else(|| {
        trigger_event.inner().source_object().and_then(|id| {
            game.object(id)
                .map(|object| ObjectSnapshot::from_object(object, game))
        })
    }) {
        tagged.insert(crate::tag::TagKey::from("triggering_source"), vec![source]);
    }
    if let Some(revealed) = trigger_event.downcast::<crate::events::CardRevealedEvent>()
        && let Some(snapshot) = revealed.snapshot.clone()
    {
        tagged.insert(
            crate::tag::TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
            vec![snapshot],
        );
    }
    if let Some(cast) = trigger_event.downcast::<crate::events::SpellCastEvent>()
        && let Some(spell) = game.object(cast.spell)
        && let Some(snapshots) = spell
            .cast_tagged_objects
            .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
        && !snapshots.is_empty()
    {
        tagged.insert(
            crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG),
            snapshots.clone(),
        );
    }
    if include_other_attackers
        && let Some(attacked) =
            trigger_event.downcast::<crate::events::combat::CreatureAttackedEvent>()
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
    if let Some(action_event) = trigger_event.downcast::<crate::events::KeywordActionEvent>() {
        for (tag, snapshots) in &action_event.object_tags {
            tagged
                .entry(tag.clone())
                .or_default()
                .extend(snapshots.clone());
        }
    }
    if let Some(zone_change_event) = trigger_event.downcast::<crate::events::ZoneChangeEvent>() {
        for (tag, snapshots) in &zone_change_event.object_tags {
            tagged
                .entry(tag.clone())
                .or_default()
                .extend(snapshots.clone());
        }
    }
    tagged
}

fn trigger_registry_key(game: &GameState) -> TriggerRegistryKey {
    TriggerRegistryKey {
        effects_revision: game.effect_store.continuous_effects.revision(),
        mutation_revision: game.mutation_revision(),
        zone_revision: game.zone_revisions().all,
        continuous_context_revision: game.continuous_context_revision(),
        turn_number: game.turn.turn_number,
        active_player: game.turn.active_player,
        phase: game.turn.phase,
        step: game.turn.step,
        combat_phases_started_this_turn: game.turn_store.combat_phases_started_this_turn,
    }
}

fn build_trigger_registry(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    key: TriggerRegistryKey,
) -> TriggerRegistry {
    let mut by_kind: FxMap<EventKind, Vec<TriggerSubscriber>> = FxMap::default();
    let mut source_local_by_kind: FxMap<EventKind, FxMap<ObjectId, Vec<TriggerSubscriber>>> =
        FxMap::default();
    let mut wildcard = Vec::new();
    let mut ordinal = 0u32;

    // A registry rebuild needs every permanent's current abilities. Prime the
    // shared batch evaluator once so a dirty continuous state does not fall
    // back to one full layer/dependency pass per battlefield object.
    view.prewarm_characteristics(&game.battlefield);

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_abilities = view
            .abilities_rc(obj_id)
            .unwrap_or_else(|| Rc::new(obj.abilities_vec()));

        for (ability_index, ability) in calculated_abilities.iter().enumerate() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&obj.zone) {
                continue;
            }
            if !presentation_labeled_trigger_is_active(game, obj, trigger_ability) {
                continue;
            }

            let subscriber = TriggerSubscriber {
                ordinal,
                source: obj_id,
                ability_index,
            };
            ordinal = ordinal.saturating_add(1);

            if let Some(kinds) = trigger_ability.trigger.subscribed_kinds() {
                for kind in kinds {
                    if trigger_ability.trigger.source_must_match_event_object(kind) {
                        source_local_by_kind
                            .entry(kind)
                            .or_default()
                            .entry(obj_id)
                            .or_default()
                            .push(subscriber);
                    } else {
                        by_kind.entry(kind).or_default().push(subscriber);
                    }
                }
            } else {
                wildcard.push(subscriber);
            }
        }
    }

    TriggerRegistry {
        key,
        by_kind,
        source_local_by_kind,
        wildcard,
    }
}

fn battlefield_trigger_registry(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> TriggerRegistry {
    let key = trigger_registry_key(game);
    game.cached_trigger_registry(key, || build_trigger_registry(game, view, key))
}

fn check_battlefield_trigger_subscriber(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
    subscriber: TriggerSubscriber,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let obj_id = subscriber.source;
    let Some(obj) = game.object(obj_id) else {
        return;
    };

    let controller = view
        .calculated_characteristics(obj_id)
        .map(|chars| chars.controller)
        .unwrap_or_else(|| game.controller_of(obj));
    let ctx = TriggerContext::for_source(obj_id, controller, game);

    let calculated_abilities = view
        .abilities_rc(obj_id)
        .unwrap_or_else(|| Rc::new(obj.abilities_vec()));
    let Some(ability) = calculated_abilities.get(subscriber.ability_index) else {
        return;
    };
    let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
        return;
    };

    if !ability.functions_in(&obj.zone) {
        return;
    }
    if !presentation_labeled_trigger_is_active(game, obj, trigger_ability) {
        return;
    }
    if skip_post_event_source_discovery(trigger_event, trigger_ability) {
        return;
    }
    if !trigger_ability.trigger.matches(trigger_event, &ctx) {
        return;
    }

    let trigger_count = trigger_ability
        .trigger
        .trigger_count_with_context(trigger_event, &ctx);
    if trigger_count == 0 {
        return;
    }
    if is_soulbond_pair_trigger(trigger_ability)
        && !soulbond_trigger_had_eligible_pair(game, obj_id, controller)
    {
        return;
    }
    let event_value_amount = trigger_ability
        .trigger
        .event_value_amount(trigger_event, &ctx);
    let trigger_identity = compute_trigger_identity(trigger_ability);
    if let Some(ref condition) = trigger_ability.intervening_if
        && !verify_intervening_if(
            game,
            condition,
            controller,
            trigger_event,
            obj_id,
            Some(trigger_identity),
            None,
        )
    {
        return;
    }

    let entry = TriggeredAbilityEntry {
        source: obj_id,
        controller,
        x_value: trigger_entry_x_value(trigger_event, obj.x_value),
        event_value_amount,
        ability: TriggeredAbility {
            trigger: trigger_ability.trigger.clone(),
            effects: trigger_ability.effects.clone(),
            choices: trigger_ability.choices.clone(),
            intervening_if: trigger_ability.intervening_if.clone(),
            presentation_label: None,
        },
        triggering_event: trigger_event.clone(),
        source_stable_id: obj.stable_id,
        source_name: obj.name.to_string(),
        source_snapshot: None,
        tagged_objects: tagged_objects_for_matched_trigger(
            game,
            trigger_event,
            &trigger_ability.trigger,
        ),
        source_kind: TriggeredAbilitySourceKind::Object,
        trigger_identity,
    };
    for _ in 0..trigger_count {
        triggered.push(entry.clone());
    }
}

#[cfg(feature = "shadow-continuous")]
fn battlefield_trigger_subscriber_matches_event(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
    subscriber: TriggerSubscriber,
) -> bool {
    let obj_id = subscriber.source;
    let Some(obj) = game.object(obj_id) else {
        return false;
    };

    let controller = view
        .calculated_characteristics(obj_id)
        .map(|chars| chars.controller)
        .unwrap_or_else(|| game.controller_of(obj));
    let ctx = TriggerContext::for_source(obj_id, controller, game);

    let calculated_abilities = view
        .abilities_rc(obj_id)
        .unwrap_or_else(|| Rc::new(obj.abilities_vec()));
    let Some(ability) = calculated_abilities.get(subscriber.ability_index) else {
        return false;
    };
    let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
        return false;
    };

    if !ability.functions_in(&obj.zone) {
        return false;
    }
    if !presentation_labeled_trigger_is_active(game, obj, trigger_ability) {
        return false;
    }
    if skip_post_event_source_discovery(trigger_event, trigger_ability) {
        return false;
    }
    if !trigger_ability.trigger.matches(trigger_event, &ctx) {
        return false;
    }

    trigger_ability
        .trigger
        .trigger_count_with_context(trigger_event, &ctx)
        != 0
}

#[cfg(feature = "shadow-continuous")]
fn legacy_battlefield_matching_trigger_subscribers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<TriggerSubscriber> {
    let mut subscribers = Vec::new();
    let mut ordinal = 0u32;

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_abilities = view
            .abilities_rc(obj_id)
            .unwrap_or_else(|| Rc::new(obj.abilities_vec()));

        for (ability_index, ability) in calculated_abilities.iter().enumerate() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&obj.zone) {
                continue;
            }
            if !presentation_labeled_trigger_is_active(game, obj, trigger_ability) {
                continue;
            }

            let subscriber = TriggerSubscriber {
                ordinal,
                source: obj_id,
                ability_index,
            };
            ordinal = ordinal.saturating_add(1);

            if battlefield_trigger_subscriber_matches_event(game, trigger_event, view, subscriber) {
                subscribers.push(subscriber);
            }
        }
    }

    subscribers
}

#[cfg(feature = "shadow-continuous")]
fn assert_trigger_registry_matches_legacy_scan(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
    registry: &TriggerRegistry,
) {
    let indexed: Vec<_> = registry
        .subscribers_for(trigger_event.kind(), trigger_event.object_id())
        .into_iter()
        .filter(|&subscriber| {
            battlefield_trigger_subscriber_matches_event(game, trigger_event, view, subscriber)
        })
        .collect();
    let legacy = legacy_battlefield_matching_trigger_subscribers(game, trigger_event, view);

    assert_eq!(
        indexed,
        legacy,
        "trigger registry diverged from legacy battlefield scan for {:?}; registry key {:?}",
        trigger_event.kind(),
        registry.key
    );
}

fn presentation_labeled_snapshot_trigger_is_active(
    game: &GameState,
    source: &ObjectSnapshot,
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    let Some(label) = triggered.presentation_label.as_ref() else {
        return true;
    };
    match label {
        PresentationLabel::CaseSolved => return game.is_case_solved(source.object_id),
        PresentationLabel::CaseToSolve => return !game.is_case_solved(source.object_id),
        _ => {}
    }
    let Some(level) = (match label {
        PresentationLabel::AbilityWord(label) => label
            .strip_prefix("__ironsmith_class_level:")
            .and_then(|level| level.parse::<u32>().ok()),
        _ => None,
    }) else {
        return true;
    };

    source
        .counters
        .get(&crate::CounterType::Level)
        .copied()
        .unwrap_or(0)
        >= level.saturating_sub(1)
}

fn skip_post_event_source_discovery(
    trigger_event: &TriggerEvent,
    trigger_ability: &TriggeredAbility,
) -> bool {
    trigger_ability.trigger.looks_back_for_source(trigger_event)
}

fn collect_lookback_source_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    for source_snapshot in trigger_event.lookback_source_snapshots() {
        for ability in source_snapshot.abilities.iter() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&source_snapshot.zone) {
                continue;
            }
            if !trigger_ability.trigger.looks_back_for_source(trigger_event) {
                continue;
            }
            if !presentation_labeled_snapshot_trigger_is_active(
                game,
                source_snapshot,
                trigger_ability,
            ) {
                continue;
            }

            let filter_ctx = lookback_source_filter_context(game, trigger_event, source_snapshot);
            let ctx = TriggerContext::new(
                source_snapshot.object_id,
                source_snapshot.controller,
                filter_ctx,
                game,
            );
            if !trigger_ability.trigger.matches(trigger_event, &ctx) {
                continue;
            }

            let trigger_count = trigger_ability
                .trigger
                .trigger_count_with_context(trigger_event, &ctx);
            if trigger_count == 0 {
                continue;
            }
            let event_value_amount = trigger_ability
                .trigger
                .event_value_amount(trigger_event, &ctx);
            let trigger_identity = compute_trigger_identity(trigger_ability);
            if let Some(ref condition) = trigger_ability.intervening_if
                && !verify_intervening_if(
                    game,
                    condition,
                    source_snapshot.controller,
                    trigger_event,
                    source_snapshot.object_id,
                    Some(trigger_identity),
                    None,
                )
            {
                continue;
            }

            let dynamic_soulshift_x = captured_dynamic_soulshift_x_value(
                game,
                trigger_event,
                source_snapshot.controller,
                source_snapshot.object_id,
                trigger_ability,
            );
            let entry = TriggeredAbilityEntry {
                source: source_snapshot.object_id,
                controller: source_snapshot.controller,
                x_value: dynamic_soulshift_x
                    .or_else(|| trigger_entry_x_value(trigger_event, source_snapshot.x_value)),
                event_value_amount,
                ability: queued_triggered_ability(trigger_ability, dynamic_soulshift_x),
                triggering_event: trigger_event.clone(),
                source_stable_id: source_snapshot.stable_id,
                source_name: source_snapshot.name.to_string(),
                source_snapshot: Some(source_snapshot.clone()),
                tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                source_kind: TriggeredAbilitySourceKind::Object,
                trigger_identity,
            };
            for _ in 0..trigger_count {
                triggered.push(entry.clone());
            }
        }
    }
}

fn lookback_source_filter_context(
    game: &GameState,
    trigger_event: &TriggerEvent,
    source_snapshot: &ObjectSnapshot,
) -> crate::target::FilterContext {
    let mut filter_ctx =
        game.filter_context_for(source_snapshot.controller, Some(source_snapshot.object_id));
    let Some(zone_change) = trigger_event.downcast::<crate::events::zones::ZoneChangeEvent>()
    else {
        return filter_ctx;
    };
    let Some(attached_sources) = zone_change.object_tags.get(ATTACHED_SOURCE_TAG) else {
        return filter_ctx;
    };
    if !attached_sources
        .iter()
        .any(|snapshot| snapshot.stable_id == source_snapshot.stable_id)
    {
        return filter_ctx;
    }

    let leaving_snapshots = zone_change.snapshots();
    if source_snapshot.subtypes.contains(&Subtype::Aura) {
        filter_ctx.tagged_objects.insert(
            crate::tag::TagKey::from("enchanted"),
            leaving_snapshots.to_vec(),
        );
    }
    if source_snapshot.subtypes.contains(&Subtype::Equipment) {
        filter_ctx.tagged_objects.insert(
            crate::tag::TagKey::from("equipped"),
            leaving_snapshots.to_vec(),
        );
    }
    filter_ctx
}

pub(crate) fn check_triggers_with_view(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<TriggeredAbilityEntry> {
    let registry = battlefield_trigger_registry(game, view);
    check_triggers_with_view_and_registry(game, trigger_event, view, &registry)
}

fn check_triggers_with_view_and_registry(
    game: &GameState,
    trigger_event: &TriggerEvent,
    view: &crate::derived_view::DerivedGameView<'_>,
    registry: &TriggerRegistry,
) -> Vec<TriggeredAbilityEntry> {
    if suppresses_creature_etb_triggers_with_effects(game, trigger_event, Some(view.effects())) {
        return Vec::new();
    }

    let mut triggered = Vec::new();
    collect_lookback_source_triggers(game, trigger_event, &mut triggered);

    #[cfg(feature = "shadow-continuous")]
    assert_trigger_registry_matches_legacy_scan(game, trigger_event, view, registry);

    for subscriber in registry.subscribers_for(trigger_event.kind(), trigger_event.object_id()) {
        check_battlefield_trigger_subscriber(game, trigger_event, view, subscriber, &mut triggered);
    }

    // A permanent can see itself being sacrificed. The sacrifice event carries
    // the pre-move snapshot, so use the same battlefield-LKI rules as LTB
    // triggers for abilities like "Whenever you sacrifice a green creature".
    if trigger_event.kind() == crate::events::traits::EventKind::Sacrifice
        && let Some(sacrifice) =
            trigger_event.downcast::<crate::events::permanents::SacrificeEvent>()
        && let Some(snapshot) = sacrifice.snapshot.as_ref()
        && !game.battlefield.contains(&snapshot.object_id)
    {
        for ability in snapshot.abilities.iter() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };

            if !ability.functions_in(&Zone::Battlefield) {
                continue;
            }

            let ctx = TriggerContext::for_source(snapshot.object_id, snapshot.controller, game);
            if trigger_ability.trigger.matches(trigger_event, &ctx) {
                let trigger_count = trigger_ability
                    .trigger
                    .trigger_count_with_context(trigger_event, &ctx);
                if trigger_count == 0 {
                    continue;
                }
                let event_value_amount = trigger_ability
                    .trigger
                    .event_value_amount(trigger_event, &ctx);
                let trigger_identity = compute_trigger_identity(trigger_ability);
                if let Some(ref condition) = trigger_ability.intervening_if
                    && !verify_intervening_if(
                        game,
                        condition,
                        snapshot.controller,
                        trigger_event,
                        snapshot.object_id,
                        Some(trigger_identity),
                        None,
                    )
                {
                    continue;
                }

                let dynamic_soulshift_x = captured_dynamic_soulshift_x_value(
                    game,
                    trigger_event,
                    snapshot.controller,
                    snapshot.object_id,
                    trigger_ability,
                );
                let entry = TriggeredAbilityEntry {
                    source: snapshot.object_id,
                    controller: snapshot.controller,
                    x_value: dynamic_soulshift_x
                        .or_else(|| trigger_entry_x_value(trigger_event, snapshot.x_value)),
                    event_value_amount,
                    ability: queued_triggered_ability(trigger_ability, dynamic_soulshift_x),
                    triggering_event: trigger_event.clone(),
                    source_stable_id: snapshot.stable_id,
                    source_name: snapshot.name.to_string(),
                    source_snapshot: Some(snapshot.clone()),
                    tagged_objects: tagged_objects_for_matched_trigger(
                        game,
                        trigger_event,
                        &trigger_ability.trigger,
                    ),
                    source_kind: TriggeredAbilitySourceKind::Object,
                    trigger_identity,
                };
                for _ in 0..trigger_count {
                    triggered.push(entry.clone());
                }
            }
        }
    }

    // Some discard triggers function from hand and must be checked using the
    // card's last-known information from immediately before it was discarded.
    if trigger_event.kind() == crate::events::traits::EventKind::CardDiscarded
        && let Some(snapshot) = trigger_event.snapshot()
    {
        for ability in snapshot.abilities.iter() {
            let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&Zone::Hand) {
                continue;
            }

            let ctx = TriggerContext::for_source(snapshot.object_id, snapshot.controller, game);
            if trigger_ability.trigger.matches(trigger_event, &ctx) {
                let trigger_count = trigger_ability
                    .trigger
                    .trigger_count_with_context(trigger_event, &ctx);
                if trigger_count == 0 {
                    continue;
                }
                let event_value_amount = trigger_ability
                    .trigger
                    .event_value_amount(trigger_event, &ctx);
                let trigger_identity = compute_trigger_identity(trigger_ability);
                if let Some(ref condition) = trigger_ability.intervening_if
                    && !verify_intervening_if(
                        game,
                        condition,
                        snapshot.controller,
                        trigger_event,
                        snapshot.object_id,
                        Some(trigger_identity),
                        None,
                    )
                {
                    continue;
                }

                let entry = TriggeredAbilityEntry {
                    source: snapshot.object_id,
                    controller: snapshot.controller,
                    x_value: trigger_entry_x_value(trigger_event, snapshot.x_value),
                    event_value_amount,
                    ability: TriggeredAbility {
                        trigger: trigger_ability.trigger.clone(),
                        effects: trigger_ability.effects.clone(),
                        choices: trigger_ability.choices.clone(),
                        intervening_if: trigger_ability.intervening_if.clone(),
                        presentation_label: None,
                    },
                    triggering_event: trigger_event.clone(),
                    source_stable_id: snapshot.stable_id,
                    source_name: snapshot.name.to_string(),
                    source_snapshot: Some(snapshot.clone()),
                    tagged_objects: tagged_objects_for_matched_trigger(
                        game,
                        trigger_event,
                        &trigger_ability.trigger,
                    ),
                    source_kind: TriggeredAbilitySourceKind::Object,
                    trigger_identity,
                };
                for _ in 0..trigger_count {
                    triggered.push(entry.clone());
                }
            }
        }
    }

    // Check objects in all public non-battlefield zones.
    for_each_public_nonbattlefield_trigger_object_id(game, |obj_id| {
        check_triggers_in_zone(game, obj_id, trigger_event, view, &mut triggered);
    });

    // Hand is hidden, but some mechanics (for example Miracle) legitimately trigger there.
    for_each_hidden_trigger_object_id(game, |obj_id| {
        check_triggers_in_zone(game, obj_id, trigger_event, view, &mut triggered);
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
        let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
        let native_cascade_count = view
            .static_abilities_rc(cast.spell)
            .map(|abilities| {
                abilities
                    .iter()
                    .filter(|static_ability| {
                        if static_ability.id() == crate::static_abilities::StaticAbilityId::Cascade
                        {
                            return true;
                        }
                        if let Some(spec) = static_ability.conditional_spell_keyword_spec()
                            && spec.keyword
                                == crate::static_abilities::ConditionalSpellKeywordKind::Cascade
                        {
                            return crate::static_abilities::conditional_spell_keyword_active(
                                spec,
                                game,
                                cast.caster,
                            );
                        }
                        false
                    })
                    .count()
            })
            .unwrap_or(0);
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
                presentation_label: None,
            };
            let trigger_identity = compute_trigger_identity(&ability);

            for _ in 0..cascade_count {
                triggered.push(TriggeredAbilityEntry {
                    source: cast.spell,
                    controller: cast.caster,
                    x_value: entry.x_value,
                    event_value_amount: None,
                    ability: ability.clone(),
                    triggering_event: trigger_event.clone(),
                    source_stable_id: obj.stable_id,
                    source_name: obj.name.to_string(),
                    source_snapshot: None,
                    tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                    source_kind: TriggeredAbilitySourceKind::Object,
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
                presentation_label: None,
            };
            let trigger_identity = compute_trigger_identity(&ability);

            triggered.push(TriggeredAbilityEntry {
                source: cast.spell,
                controller: cast.caster,
                x_value: entry.x_value,
                event_value_amount: None,
                ability,
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.to_string(),
                source_snapshot: None,
                tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                source_kind: TriggeredAbilitySourceKind::Object,
                trigger_identity,
            });
        }
    }

    // Conspire granted by a static ability is modeled as a dynamic optional cost.
    // Printed conspire cards already carry their own trigger, so only the
    // granted-cost label is handled here.
    if trigger_event.kind() == crate::events::traits::EventKind::SpellCast
        && let Some(cast) = trigger_event.downcast::<crate::events::spells::SpellCastEvent>()
        && let Some(entry) = game.stack.iter().find(|e| e.object_id == cast.spell)
    {
        let times = entry
            .optional_costs_paid
            .times_paid_label("Granted Conspire");
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
                presentation_label: None,
            };
            let trigger_identity = compute_trigger_identity(&ability);

            triggered.push(TriggeredAbilityEntry {
                source: cast.spell,
                controller: cast.caster,
                x_value: entry.x_value,
                event_value_amount: None,
                ability,
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.to_string(),
                source_snapshot: None,
                tagged_objects: tagged_objects_for_trigger_event(game, trigger_event),
                source_kind: TriggeredAbilitySourceKind::Object,
                trigger_identity,
            });
        }
    }

    add_monarch_designation_triggers(game, trigger_event, &mut triggered);
    add_initiative_designation_triggers(game, trigger_event, &mut triggered);
    add_ring_designation_triggers(game, trigger_event, &mut triggered);
    add_speed_increase_triggers(game, trigger_event, &mut triggered);
    remove_suppressed_triggers(game, view, &mut triggered);
    append_additional_trigger_copies(game, view, &mut triggered);

    triggered
}

fn presentation_labeled_trigger_is_active(
    game: &GameState,
    source: &crate::object::Object,
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    let Some(label) = triggered.presentation_label.as_ref() else {
        return true;
    };
    match label {
        PresentationLabel::CaseSolved => return game.is_case_solved(source.id),
        PresentationLabel::CaseToSolve => return !game.is_case_solved(source.id),
        _ => {}
    }
    let Some(level) = (match label {
        PresentationLabel::AbilityWord(label) => label
            .strip_prefix("__ironsmith_class_level:")
            .and_then(|level| level.parse::<u32>().ok()),
        _ => None,
    }) else {
        return true;
    };

    source
        .counters
        .get(&crate::CounterType::Level)
        .copied()
        .unwrap_or(0)
        >= level.saturating_sub(1)
}

fn add_speed_increase_triggers(
    game: &GameState,
    trigger_event: &TriggerEvent,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    if trigger_event.kind() != crate::events::traits::EventKind::LifeLoss {
        return;
    }
    let Some(loss) = trigger_event.downcast::<crate::events::LifeLossEvent>() else {
        return;
    };
    if loss.amount == 0 {
        return;
    }

    let controller = game.turn.active_player;
    if loss.player == controller
        || game.speed_increase_triggered_this_turn(controller)
        || !matches!(game.player_speed(controller), Some(1..=3))
    {
        return;
    }

    let ability = TriggeredAbility {
        trigger: Trigger::custom(
            "speed-inherent",
            "Whenever one or more opponents lose life during your turn".to_string(),
        ),
        effects: ResolutionProgram::from_effects(vec![Effect::increase_speed(
            crate::effect::Value::Fixed(1),
            PlayerFilter::You,
        )]),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };
    let trigger_identity = compute_trigger_identity(&ability);
    let source = speed_rule_source_id();

    triggered.push(TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability,
        triggering_event: trigger_event.clone(),
        source_stable_id: StableId::from_raw(0),
        source_name: SPEED_RULE_SOURCE_NAME.to_string(),
        source_snapshot: None,
        tagged_objects: HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
        trigger_identity,
    });
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
    controller: PlayerId,
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
            controller,
            &trigger_event,
            obj.id,
            Some(trigger_identity),
            None,
        ) {
            continue;
        }

        active.insert(key);
        if game
            .effect_store
            .active_state_trigger_conditions
            .contains(&key)
        {
            continue;
        }

        let tagged_objects = tagged_objects_for_trigger_event(game, &trigger_event);
        triggered.push(TriggeredAbilityEntry {
            source: obj.id,
            controller,
            x_value: trigger_entry_x_value(&trigger_event, obj.x_value),
            event_value_amount: None,
            ability: TriggeredAbility {
                trigger: trigger_ability.trigger.clone(),
                effects: trigger_ability.effects.clone(),
                choices: trigger_ability.choices.clone(),
                intervening_if: trigger_ability.intervening_if.clone(),
                presentation_label: None,
            },
            triggering_event: trigger_event,
            source_stable_id: obj.stable_id,
            source_name: obj.name.to_string(),
            source_snapshot: None,
            tagged_objects,
            source_kind: TriggeredAbilitySourceKind::Object,
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
            .unwrap_or_else(|| Rc::new(obj.abilities_vec()));
        let controller = view
            .calculated_characteristics(obj_id)
            .map(|chars| chars.controller)
            .unwrap_or_else(|| game.controller_of(obj));
        collect_state_triggers_for_object(
            game,
            obj,
            controller,
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
                game.controller_of(obj),
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
                game.controller_of(obj),
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
    if game.effect_store.delayed_triggers.is_empty() {
        return Vec::new();
    }

    if suppresses_creature_etb_triggers(game, trigger_event) {
        return Vec::new();
    }

    let mut triggered = Vec::new();
    let mut to_remove = Vec::new();

    for (idx, delayed) in game.effect_store.delayed_triggers.iter().enumerate() {
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
            let ctx = TriggerContext::for_delayed_source(
                source,
                delayed.controller,
                game,
                &delayed.tagged_objects,
            );
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
                .or_else(|| game.object(ability_source).map(|o| o.name.to_string()))
                .or_else(|| {
                    game.find_object_by_stable_id(source_stable_id)
                        .and_then(|id| game.object(id))
                        .map(|o| o.name.to_string())
                })
                .or_else(|| {
                    if trigger_event.object_id() == Some(ability_source) {
                        trigger_event
                            .snapshot()
                            .map(|snapshot| snapshot.name.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "Delayed Trigger".to_string());

            triggered.push(TriggeredAbilityEntry {
                source: ability_source,
                controller: delayed.controller,
                x_value: delayed.x_value,
                event_value_amount: delayed.trigger.event_value_amount(trigger_event, &ctx),
                ability: TriggeredAbility {
                    trigger: delayed.trigger.clone(),
                    effects: delayed.effects.clone(),
                    choices: delayed.choices.clone(),
                    intervening_if: None,
                    presentation_label: None,
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
                source_kind: TriggeredAbilitySourceKind::Object,
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
        game.effect_store.delayed_triggers.retain(|_| {
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
    view: &crate::derived_view::DerivedGameView<'_>,
    triggered: &mut Vec<TriggeredAbilityEntry>,
) {
    let Some(obj) = game.object(obj_id) else {
        return;
    };

    let ctx = TriggerContext::for_source(obj_id, game.controller_of(obj), game);

    let calculated_abilities = view
        .abilities_rc(obj_id)
        .unwrap_or_else(|| Rc::new(obj.abilities_vec()));

    for ability in calculated_abilities.iter() {
        let AbilityKind::Triggered(trigger_ability) = &ability.kind else {
            continue;
        };

        if !ability.functions_in(&obj.zone) {
            continue;
        }

        if skip_post_event_source_discovery(trigger_event, trigger_ability) {
            continue;
        }

        if trigger_ability.trigger.matches(trigger_event, &ctx) {
            let trigger_count = trigger_ability
                .trigger
                .trigger_count_with_context(trigger_event, &ctx);
            if trigger_count == 0 {
                continue;
            }
            let event_value_amount = trigger_ability
                .trigger
                .event_value_amount(trigger_event, &ctx);
            let trigger_identity = compute_trigger_identity(trigger_ability);
            if let Some(ref condition) = trigger_ability.intervening_if
                && !verify_intervening_if(
                    game,
                    condition,
                    game.controller_of(obj),
                    trigger_event,
                    obj_id,
                    Some(trigger_identity),
                    None,
                )
            {
                continue;
            }

            let entry = TriggeredAbilityEntry {
                source: obj_id,
                controller: game.controller_of(obj),
                x_value: trigger_entry_x_value(trigger_event, obj.x_value),
                event_value_amount,
                ability: TriggeredAbility {
                    trigger: trigger_ability.trigger.clone(),
                    effects: trigger_ability.effects.clone(),
                    choices: trigger_ability.choices.clone(),
                    intervening_if: trigger_ability.intervening_if.clone(),
                    presentation_label: None,
                },
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.to_string(),
                source_snapshot: None,
                tagged_objects: tagged_objects_for_matched_trigger(
                    game,
                    trigger_event,
                    &trigger_ability.trigger,
                ),
                source_kind: TriggeredAbilitySourceKind::Object,
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
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) => true,
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
        PlayerFilter::LowestLifeTied => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.life)
            .min()
            .is_some_and(|min_life| {
                game.player(player)
                    .is_some_and(|candidate| candidate.is_in_game() && candidate.life == min_life)
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
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            player_filter_matches_with_context(base, player, controller, game, defending_player)
                && game.player(player).is_some_and(|candidate| {
                    let your_hand = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
                    candidate.hand.len() >= your_hand.saturating_add(*count as usize)
                })
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            player_filter_matches_with_context(base, player, controller, game, defending_player)
                && game
                    .player(player)
                    .zip(game.player(controller))
                    .is_some_and(|(candidate, you)| candidate.life > you.life)
        }
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            player_filter_matches_with_context(base, player, controller, game, defending_player)
                && game.has_max_speed(player) == *has_max_speed
        }
        PlayerFilter::CastCardTypeThisTurn(card_type) => game
            .turn_store
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
                .is_some_and(|obj| player == game.controller_of(obj)),
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
                .is_some_and(|obj| player == game.controller_of(obj)),
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
    optional_costs_paid: Option<&crate::cost::OptionalCostsPaid>,
) -> bool {
    let defending_player = if event.kind() == crate::events::traits::EventKind::CreatureAttacked {
        event
            .downcast::<crate::events::combat::CreatureAttackedEvent>()
            .and_then(|attacked| match attacked.target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => game
                    .object(planeswalker_id)
                    .map(|planeswalker| game.controller_of(planeswalker)),
            })
    } else if event.kind() == crate::events::traits::EventKind::CreatureAttackedAndUnblocked {
        event
            .downcast::<crate::events::combat::CreatureAttackedAndUnblockedEvent>()
            .and_then(|attacked| match attacked.target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => game
                    .object(planeswalker_id)
                    .map(|planeswalker| game.controller_of(planeswalker)),
            })
    } else if event.kind() == crate::events::traits::EventKind::CreatureBecameBlocked {
        event
            .downcast::<crate::events::combat::CreatureBecameBlockedEvent>()
            .and_then(|blocked| blocked.attack_target)
            .and_then(|target| match target {
                crate::triggers::AttackEventTarget::Player(player_id) => Some(player_id),
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id) => game
                    .object(planeswalker_id)
                    .map(|planeswalker| game.controller_of(planeswalker)),
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
        iterated_player: None,
        triggering_event: Some(event),
        trigger_identity,
        ability_index: None,
        options: Default::default(),
    };
    evaluate_intervening_if_condition(game, condition, &eval_ctx, optional_costs_paid)
}

fn evaluate_intervening_if_condition(
    game: &GameState,
    condition: &crate::ConditionExpr,
    eval_ctx: &crate::condition_eval::ExternalEvaluationContext<'_>,
    optional_costs_paid: Option<&crate::cost::OptionalCostsPaid>,
) -> bool {
    match condition {
        crate::effect::Condition::Not(inner) => {
            !evaluate_intervening_if_condition(game, inner, eval_ctx, optional_costs_paid)
        }
        crate::effect::Condition::And(left, right) => {
            evaluate_intervening_if_condition(game, left, eval_ctx, optional_costs_paid)
                && evaluate_intervening_if_condition(game, right, eval_ctx, optional_costs_paid)
        }
        crate::effect::Condition::Or(left, right) => {
            evaluate_intervening_if_condition(game, left, eval_ctx, optional_costs_paid)
                || evaluate_intervening_if_condition(game, right, eval_ctx, optional_costs_paid)
        }
        crate::effect::Condition::ThisSpellWasKicked => optional_costs_paid.map_or_else(
            || crate::condition_eval::evaluate_condition_external(game, condition, eval_ctx),
            crate::cost::OptionalCostsPaid::was_kicked,
        ),
        crate::effect::Condition::ThisSpellPaidLabel(label) => optional_costs_paid.map_or_else(
            || crate::condition_eval::evaluate_condition_external(game, condition, eval_ctx),
            |paid| paid.was_paid_label(label.clone()),
        ),
        _ => crate::condition_eval::evaluate_condition_external(game, condition, eval_ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::combat_state::AttackTarget;
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::events::cause::EventCause;
    use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent, CreatureBlockedEvent};
    use crate::events::other::{BecameMonstrousEvent, ControlChangedEvent, PlayerLosesGameEvent};
    use crate::events::spells::{AbilityActivatedEvent, BecomesTargetedEvent, SpellCastEvent};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::ChooseSpec;
    use crate::types::{CardType, Subtype};
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

    fn add_battlefield_trigger(
        game: &mut GameState,
        source: crate::ids::ObjectId,
        trigger: Trigger,
    ) {
        game.object_mut(source)
            .expect("source should exist")
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger,
                    effects: vec![Effect::gain_life(2)].into(),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
    }

    fn add_conditional_battlefield_trigger_grant(
        game: &mut GameState,
        source: crate::ids::ObjectId,
        controller: PlayerId,
        condition: crate::ConditionExpr,
    ) {
        let granted_trigger = crate::ability::Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_attacks(),
                effects: vec![Effect::gain_life(2)].into(),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                source,
                controller,
                crate::continuous::EffectTarget::Source,
                crate::continuous::Modification::AddAbilityGeneric(granted_trigger),
            )
            .with_condition(condition),
        );
    }

    #[test]
    fn empty_delayed_trigger_check_skips_creature_etb_suppression_scan() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let creatures = (0..96)
            .map(|index| {
                make_battlefield_creature(&mut game, alice, &format!("Layered Creature {index}"))
            })
            .collect::<Vec<_>>();
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                creatures[0],
                alice,
                crate::continuous::EffectTarget::AllCreatures,
                crate::continuous::Modification::AddAbility(StaticAbility::flying()),
            ));
        game.refresh_continuous_state();
        assert!(game.effect_store.delayed_triggers.is_empty());

        let event = TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creatures[1],
                Zone::Stack,
                Zone::Battlefield,
                EventCause::effect(),
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let before = game.work_counters();

        assert!(check_delayed_triggers(&mut game, &event).is_empty());

        let after = game.work_counters();
        assert_eq!(
            after.derived_view_rebuilds, before.derived_view_rebuilds,
            "an empty delayed-trigger store must return before building a suppression view"
        );
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "an empty delayed-trigger store must not calculate battlefield characteristics"
        );
        assert_eq!(
            after.dependency_sorts, before.dependency_sorts,
            "an empty delayed-trigger store must not enter dependency sorting"
        );
    }

    fn make_battlefield_artifact(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[derive(Debug, Clone)]
    struct ControlChangedLookbackTrigger;

    impl crate::triggers::matcher_trait::TriggerMatcher for ControlChangedLookbackTrigger {
        fn matches(
            &self,
            event: &TriggerEvent,
            _ctx: &crate::triggers::matcher_trait::TriggerContext,
        ) -> bool {
            event
                .downcast::<ControlChangedEvent>()
                .is_some_and(|event| event.previous_controller != event.new_controller)
        }

        fn display(&self) -> String {
            "Whenever a player loses control of an object".to_string()
        }

        fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
            event.kind() == crate::events::traits::EventKind::ControlChanged
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    fn dungeon_room_trigger_entry(
        controller: PlayerId,
        event: &TriggerEvent,
    ) -> TriggeredAbilityEntry {
        let source = crate::ids::ObjectId::from_raw(u64::MAX - 17);
        let ability = TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects: ResolutionProgram::default(),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };
        let trigger_identity = compute_trigger_identity(&ability);
        TriggeredAbilityEntry {
            source,
            controller,
            x_value: None,
            event_value_amount: None,
            ability,
            triggering_event: event.clone(),
            source_stable_id: StableId::from(source),
            source_name: "Cave Entrance".to_string(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            source_kind: TriggeredAbilitySourceKind::DungeonRoom,
            trigger_identity,
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    fn dungeon_delver_game_with_commander() -> (GameState, PlayerId, crate::ids::ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let commander = make_battlefield_creature(&mut game, alice, "Dungeon Delver Commander");
        game.set_as_commander(commander, alice);

        let dungeon_delver = CardDefinitionBuilder::new(CardId::new(), "Dungeon Delver")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Background])
            .parse_text(
                "Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\"",
            )
            .expect("Dungeon Delver should parse");
        game.create_object_from_definition(&dungeon_delver, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        (game, alice, commander)
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn dungeon_delver_commander_duplicates_owned_dungeon_room_trigger() {
        let (game, alice, commander) = dungeon_delver_game_with_commander();
        let view = crate::derived_view::DerivedGameView::new(&game);
        let commander_static_abilities = view
            .static_abilities_rc(commander)
            .expect("commander should have calculated static abilities");
        assert!(
            commander_static_abilities.iter().any(|ability| {
                ability.id() == StaticAbilityId::DungeonRoomTriggerDuplication
                    && ability.trigger_duplication_spec().is_some()
            }),
            "Dungeon Delver should grant a real dungeon-room trigger duplication spec"
        );

        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let mut triggered = vec![dungeon_room_trigger_entry(alice, &event)];

        append_additional_trigger_copies(&game, &view, &mut triggered);

        assert_eq!(
            triggered.len(),
            2,
            "Dungeon Delver should add one copy for an owned dungeon room trigger"
        );
        assert!(
            triggered
                .iter()
                .all(|entry| entry.source_kind == TriggeredAbilitySourceKind::DungeonRoom),
            "the copied trigger should preserve dungeon room source metadata"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn dungeon_delver_does_not_duplicate_unowned_or_non_room_triggers() {
        let (game, alice, _commander) = dungeon_delver_game_with_commander();
        let bob = PlayerId::from_index(1);
        let view = crate::derived_view::DerivedGameView::new(&game);
        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        let mut unowned_room = vec![dungeon_room_trigger_entry(bob, &event)];
        append_additional_trigger_copies(&game, &view, &mut unowned_room);
        assert_eq!(
            unowned_room.len(),
            1,
            "Dungeon Delver should not copy another player's dungeon room trigger"
        );

        let mut object_trigger = dungeon_room_trigger_entry(alice, &event);
        object_trigger.source_kind = TriggeredAbilitySourceKind::Object;
        let mut object_triggers = vec![object_trigger];
        append_additional_trigger_copies(&game, &view, &mut object_triggers);
        assert_eq!(
            object_triggers.len(),
            1,
            "Dungeon Delver should not copy ordinary object triggers"
        );
    }

    fn unbound_flourishing_like_definition() -> crate::cards::CardDefinition {
        let mut permanent_x_spell = ObjectFilter::permanent_card();
        permanent_x_spell.zone = Some(Zone::Stack);
        permanent_x_spell.has_x_in_cost = true;
        let mut instant_or_sorcery_x_spell = ObjectFilter::instant_or_sorcery();
        instant_or_sorcery_x_spell.has_x_in_cost = true;
        let mut activated_ability_with_x = ObjectFilter::default();
        activated_ability_with_x.has_x_in_cost = true;

        CardDefinitionBuilder::new(CardId::from_raw(88101), "Unbound Flourishing")
            .card_types(vec![CardType::Enchantment])
            .with_trigger(
                Trigger::spell_cast(Some(permanent_x_spell), PlayerFilter::You),
                Vec::new(),
            )
            .with_trigger(
                Trigger::either(
                    Trigger::spell_cast(Some(instant_or_sorcery_x_spell), PlayerFilter::You),
                    Trigger::ability_activated_qualified(
                        PlayerFilter::You,
                        activated_ability_with_x,
                        false,
                        false,
                    ),
                ),
                Vec::new(),
            )
            .build()
    }

    fn stack_spell(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        card_type: CardType,
        has_x_cost: bool,
        x_value: Option<u32>,
    ) -> crate::ids::ObjectId {
        let mana_cost = if has_x_cost {
            ManaCost::from_pips(vec![vec![ManaSymbol::X]])
        } else {
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]])
        };
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(mana_cost)
            .card_types(vec![card_type])
            .build();
        let spell = game.create_object_from_card(&card, controller, Zone::Stack);
        game.object_mut(spell).expect("spell object exists").x_value = x_value;
        let mut entry = crate::game_state::StackEntry::new(spell, controller);
        entry.x_value = x_value;
        game.push_to_stack(entry);
        spell
    }

    #[test]
    fn unbound_flourishing_permanent_spell_trigger_requires_x_and_captures_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let unbound = unbound_flourishing_like_definition();
        game.create_object_from_definition(&unbound, alice, Zone::Battlefield);
        let hydra = stack_spell(
            &mut game,
            alice,
            "Hydra Spell",
            CardType::Creature,
            true,
            Some(3),
        );

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                SpellCastEvent::new(hydra, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1, "expected permanent X spell trigger");
        assert_eq!(triggered[0].source_name, "Unbound Flourishing");
    }

    #[test]
    fn unbound_flourishing_copy_trigger_matches_x_spell_and_x_ability_only() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let unbound = unbound_flourishing_like_definition();
        let source = game.create_object_from_definition(&unbound, alice, Zone::Battlefield);
        let x_sorcery = stack_spell(
            &mut game,
            alice,
            "X Sorcery",
            CardType::Sorcery,
            true,
            Some(4),
        );
        let plain_sorcery = stack_spell(
            &mut game,
            alice,
            "Plain Sorcery",
            CardType::Sorcery,
            false,
            None,
        );

        let x_spell_triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                SpellCastEvent::new(x_sorcery, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert_eq!(x_spell_triggered.len(), 1, "expected X sorcery trigger");

        let plain_spell_triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                SpellCastEvent::new(plain_sorcery, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert!(
            plain_spell_triggered.is_empty(),
            "plain sorcery should not satisfy X-cost trigger"
        );

        let x_ability_triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                AbilityActivatedEvent::new(source, alice, false)
                    .with_activation_cost_has_x(true)
                    .with_x_value(Some(2)),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert_eq!(x_ability_triggered.len(), 1, "expected X ability trigger");
        assert_eq!(x_ability_triggered[0].x_value, Some(2));

        let plain_ability_triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                AbilityActivatedEvent::new(source, alice, false),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert!(
            plain_ability_triggered.is_empty(),
            "ability without X value should not satisfy X-cost trigger"
        );

        let x_value_without_x_cost_triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                AbilityActivatedEvent::new(source, alice, false).with_x_value(Some(2)),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert!(
            x_value_without_x_cost_triggered.is_empty(),
            "an X value alone should not satisfy activation-cost X trigger"
        );
    }

    #[test]
    fn sacrifice_event_uses_source_lki_for_self_sacrifice_triggers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_creature(&mut game, alice, "Self-Sacrifice Watcher");
        if let Some(object) = game.object_mut(source) {
            object.abilities_mut().push(crate::ability::Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::player_sacrifices(
                        PlayerFilter::You,
                        ObjectFilter::creature(),
                    ),
                    effects: vec![Effect::gain_life(2)].into(),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        }

        let snapshot = game
            .object(source)
            .map(|object| ObjectSnapshot::from_object(object, &game))
            .expect("source should exist before sacrifice");
        game.move_object_by_effect(source, Zone::Graveyard);
        let event = TriggerEvent::new_with_provenance(
            crate::events::permanents::SacrificeEvent::new(source, None)
                .with_snapshot(Some(snapshot), Some(alice)),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);

        assert_eq!(triggered.len(), 1, "expected self-sacrifice LKI trigger");
        assert_eq!(triggered[0].source_name, "Self-Sacrifice Watcher");
        assert!(triggered[0].source_snapshot.is_some());
    }

    #[test]
    fn graveyard_leave_trigger_does_not_see_source_return_from_graveyard() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::new(), "Returning Graveyard Watcher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Graveyard);
        game.object_mut(source)
            .expect("source should exist")
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::cards_leave_your_graveyard(
                        ObjectFilter::creature(),
                        true,
                        false,
                    ),
                    effects: vec![Effect::gain_life(2)].into(),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });

        let returned_source = game
            .move_object_by_effect(source, Zone::Battlefield)
            .expect("source should return from the graveyard");
        let events = game.take_pending_trigger_events();
        let graveyard_leave_event = events
            .iter()
            .find(|event| {
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|zone_change| {
                        zone_change.from == Zone::Graveyard
                            && zone_change.to == Zone::Battlefield
                            && zone_change.destination_objects().contains(&returned_source)
                    })
            })
            .expect("expected graveyard-to-battlefield event");

        let triggered = check_triggers(&game, graveyard_leave_event);

        assert!(
            triggered.is_empty(),
            "a battlefield-only graveyard-leave trigger must use pre-event source existence"
        );
    }

    #[test]
    fn ltb_trigger_uses_pre_event_source_snapshot_when_source_leaves() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Departing Watcher");
        add_battlefield_trigger(&mut game, source, Trigger::dies(ObjectFilter::creature()));
        let victim = make_battlefield_creature(&mut game, alice, "Departing Bear");

        let source_snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source exists"), &game);
        let victim_snapshot =
            ObjectSnapshot::from_object(game.object(victim).expect("victim exists"), &game);
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should move");
        game.move_object_by_effect(victim, Zone::Graveyard)
            .expect("victim should move");
        game.take_pending_trigger_events();

        let event = TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
                vec![source, victim],
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::effect(),
                vec![source_snapshot.clone(), victim_snapshot],
            ),
            crate::provenance::ProvNodeId::default(),
        )
        .with_lookback_source_snapshots(vec![source_snapshot.clone()]);

        let triggered = check_triggers(&game, &event);

        assert_eq!(triggered.len(), 1, "source should see the creature die");
        assert_eq!(triggered[0].source_stable_id, source_snapshot.stable_id);
        assert!(
            triggered[0].source_snapshot.is_some(),
            "look-back source discovery should queue from source LKI"
        );
    }

    #[test]
    fn ltb_trigger_without_lookback_payload_does_not_use_current_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Current Watcher");
        add_battlefield_trigger(&mut game, source, Trigger::dies(ObjectFilter::creature()));
        let victim = make_battlefield_creature(&mut game, alice, "Manual Bear");
        let victim_snapshot =
            ObjectSnapshot::from_object(game.object(victim).expect("victim exists"), &game);

        let event = TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                victim,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::effect(),
                Some(victim_snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);

        assert!(
            triggered.is_empty(),
            "603.10 trigger sources must come from the pre-event look-back payload"
        );
    }

    #[test]
    fn unrelated_zone_change_lki_skips_layered_trigger_registry_rebuild() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        // Model a large board whose only triggers subscribe to combat events.
        // A zone-change LKI snapshot must not make those sources relevant.
        for index in 0..128 {
            let source =
                make_battlefield_creature(&mut game, alice, &format!("Attack Watcher {index}"));
            add_battlefield_trigger(&mut game, source, Trigger::this_attacks());
        }
        let victim = make_battlefield_creature(&mut game, alice, "Departing Creature");
        game.refresh_continuous_state();
        let victim_snapshot =
            ObjectSnapshot::from_object(game.object(victim).expect("victim exists"), &game);
        let event = TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                victim,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::from_legend_rule(alice),
                Some(victim_snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let before = game.work_counters();

        let triggered = check_triggers(&game, &event);

        let after = game.work_counters();
        assert!(triggered.is_empty());
        assert_eq!(
            after.derived_view_rebuilds, before.derived_view_rebuilds,
            "an unrelated LKI payload should not build a layered trigger view"
        );
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "an unrelated LKI payload should not calculate battlefield characteristics"
        );
    }

    #[test]
    fn public_zone_to_hand_trigger_uses_pre_event_source_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Public Hand Watcher");
        add_battlefield_trigger(&mut game, source, Trigger::card_put_into_hand());
        let source_stable_id = game.object(source).expect("source exists").stable_id;

        let card = CardBuilder::new(CardId::new(), "Known Graveyard Card")
            .card_types(vec![CardType::Creature])
            .build();
        let graveyard_card = game.create_object_from_card(&card, alice, Zone::Graveyard);
        game.move_object_by_effect(graveyard_card, Zone::Hand)
            .expect("card should move to hand");
        let events = game.take_pending_trigger_events();
        let event = events
            .iter()
            .find(|event| {
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|zone_change| {
                        zone_change.from == Zone::Graveyard && zone_change.to == Zone::Hand
                    })
            })
            .expect("expected graveyard-to-hand event");

        let triggered = check_triggers(&game, event);

        assert_eq!(
            triggered.len(),
            1,
            "public object put into hand should trigger"
        );
        assert_eq!(triggered[0].source_stable_id, source_stable_id);
        assert!(
            triggered[0].source_snapshot.is_some(),
            "source should be discovered from the pre-event look-back payload"
        );
    }

    #[test]
    fn player_loses_game_trigger_uses_pre_event_source_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_artifact(&mut game, alice, "Loss Watcher");
        add_battlefield_trigger(
            &mut game,
            source,
            Trigger::player_loses_game(PlayerFilter::Opponent),
        );
        let source_stable_id = game.object(source).expect("source exists").stable_id;

        assert!(game.mark_player_lost(bob));
        let events = game.take_pending_trigger_events();
        let event = events
            .iter()
            .find(|event| event.downcast::<PlayerLosesGameEvent>().is_some())
            .expect("expected player-loses-game event");

        let triggered = check_triggers(&game, event);

        assert_eq!(triggered.len(), 1, "opponent losing should trigger");
        assert_eq!(triggered[0].controller, alice);
        assert_eq!(triggered[0].source_stable_id, source_stable_id);
        assert!(
            triggered[0].source_snapshot.is_some(),
            "player-loss triggers are CR 603.10 look-back triggers"
        );
    }

    #[test]
    fn control_change_lookback_trigger_uses_previous_controller_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_artifact(&mut game, alice, "Control Watcher");
        add_battlefield_trigger(
            &mut game,
            source,
            Trigger::new(ControlChangedLookbackTrigger),
        );

        let event = TriggerEvent::new_with_provenance(
            ControlChangedEvent::new(source, alice, bob),
            crate::provenance::ProvNodeId::default(),
        )
        .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());

        let triggered = check_triggers(&game, &event);

        assert_eq!(triggered.len(), 1, "control-change source should look back");
        assert_eq!(
            triggered[0].controller, alice,
            "controller should come from the pre-event source snapshot"
        );
        assert!(
            triggered[0].source_snapshot.is_some(),
            "control-change look-back trigger should queue from source LKI"
        );
    }

    #[test]
    fn permanent_phased_out_event_kind_trigger_uses_pre_event_source_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Phase Watcher");
        add_battlefield_trigger(
            &mut game,
            source,
            Trigger::new(crate::triggers::EventKindTrigger::new(
                crate::events::traits::EventKind::PermanentPhasedOut,
                "Whenever a permanent phases out",
            )),
        );

        game.phase_out(source);
        let events = game.take_pending_trigger_events();
        let event = events
            .iter()
            .find(|event| event.kind() == crate::events::traits::EventKind::PermanentPhasedOut)
            .expect("expected phase-out event");

        let triggered = check_triggers(&game, event);

        assert_eq!(triggered.len(), 1, "phase-out source should look back");
        assert!(triggered[0].source_snapshot.is_some());
    }

    #[test]
    fn became_unattached_event_kind_trigger_uses_pre_event_source_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Unattach Watcher");
        add_battlefield_trigger(
            &mut game,
            source,
            Trigger::new(crate::triggers::EventKindTrigger::new(
                crate::events::traits::EventKind::ObjectBecameUnattached,
                "Whenever an object becomes unattached",
            )),
        );
        let equipment = make_battlefield_artifact(&mut game, alice, "Loose Equipment");
        let creature = make_battlefield_creature(&mut game, alice, "Equipped Creature");
        assert!(
            game.attach_object_to_target(
                equipment,
                crate::object::AttachmentTarget::Object(creature),
            )
        );
        game.take_pending_trigger_events();

        assert!(game.detach_object_from_current_target(equipment));
        let events = game.take_pending_trigger_events();
        let event = events
            .iter()
            .find(|event| event.kind() == crate::events::traits::EventKind::ObjectBecameUnattached)
            .expect("expected unattached event");

        let triggered = check_triggers(&game, event);

        assert_eq!(triggered.len(), 1, "unattached source should look back");
        assert!(triggered[0].source_snapshot.is_some());
    }

    #[test]
    fn spell_countered_event_kind_trigger_uses_pre_event_source_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = make_battlefield_artifact(&mut game, alice, "Counter Watcher");
        add_battlefield_trigger(
            &mut game,
            source,
            Trigger::new(crate::triggers::EventKindTrigger::new(
                crate::events::traits::EventKind::SpellCountered,
                "Whenever a spell is countered",
            )),
        );
        let spell = stack_spell(
            &mut game,
            alice,
            "Countered Spell",
            CardType::Instant,
            false,
            None,
        );
        let spell_snapshot =
            ObjectSnapshot::from_object(game.object(spell).expect("spell exists"), &game);
        let event = TriggerEvent::new_with_provenance(
            crate::events::SpellCounteredEvent::new(spell, alice, Some(spell_snapshot)),
            crate::provenance::ProvNodeId::default(),
        )
        .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());

        let triggered = check_triggers(&game, &event);

        assert_eq!(
            triggered.len(),
            1,
            "spell-countered source should look back"
        );
        assert!(triggered[0].source_snapshot.is_some());
    }

    #[test]
    fn soulbond_does_not_trigger_without_another_unpaired_creature_at_trigger_time() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        make_battlefield_artifact(&mut game, alice, "Angel's Tomb");

        let soulbond = CardDefinitionBuilder::new(CardId::new(), "Trusted Forcemage")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .soulbond()
            .build();
        let source = game.create_object_from_definition(&soulbond, alice, Zone::Battlefield);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                crate::events::zones::ZoneChangeEvent::with_cause(
                    source,
                    Zone::Hand,
                    Zone::Battlefield,
                    EventCause::from_game_rule(),
                    None,
                ),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert!(
            triggered.is_empty(),
            "soulbond should not trigger if no other unpaired creature existed at trigger time"
        );
    }

    #[test]
    fn soulbond_triggers_when_another_unpaired_creature_exists_at_trigger_time() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        make_battlefield_creature(&mut game, alice, "Elite Vanguard");

        let soulbond = CardDefinitionBuilder::new(CardId::new(), "Trusted Forcemage")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .soulbond()
            .build();
        let source = game.create_object_from_definition(&soulbond, alice, Zone::Battlefield);

        let triggered = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                crate::events::zones::ZoneChangeEvent::with_cause(
                    source,
                    Zone::Hand,
                    Zone::Battlefield,
                    EventCause::from_game_rule(),
                    None,
                ),
                crate::provenance::ProvNodeId::default(),
            ),
        );

        assert_eq!(triggered.len(), 1, "expected one soulbond trigger");
    }

    #[test]
    fn or_spell_cast_trigger_uses_current_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        let voice_like = CardDefinitionBuilder::new(CardId::new(), "Voice-Like")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_trigger(
                Trigger::or(vec![
                    Trigger::new(crate::triggers::SpellCastTrigger::qualified(
                        None,
                        PlayerFilter::Opponent,
                        Some(PlayerFilter::You),
                        None,
                        None,
                        false,
                    )),
                    Trigger::this_dies(),
                ]),
                Vec::new(),
            )
            .build();
        let source = game.create_object_from_definition(&voice_like, bob, Zone::Battlefield);
        let aura = make_battlefield_artifact(&mut game, alice, "Control Aura");
        assert!(
            game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(source),)
        );
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                aura,
                alice,
                crate::continuous::EffectTarget::AttachedTo(aura),
                crate::continuous::Modification::ChangeController(alice),
            ));
        game.refresh_continuous_state();

        let spell = CardBuilder::new(CardId::new(), "Lightning Bolt")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, bob, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, bob));

        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let triggered = check_triggers(&game, &event);

        assert_eq!(triggered.len(), 1, "expected opponent-cast trigger");
        assert_eq!(triggered[0].controller, alice);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn attached_aura_lki_trigger_sees_enchanted_creature_die() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let creature = make_battlefield_creature(&mut game, alice, "Silvercoat Lion");
        let aura_def = CardDefinitionBuilder::new(CardId::new(), "Return Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text("Enchant creature\nWhen enchanted creature dies, return this card to its owner's hand.")
            .expect("aura death trigger should parse");
        let aura = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(creature),)
        );

        game.move_object_by_effect(creature, Zone::Graveyard);
        let events = game.take_pending_trigger_events();
        let dies_event = events
            .iter()
            .find(|event| {
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|zone_change| zone_change.is_dies())
            })
            .expect("expected creature dies event");

        let triggered = check_triggers(&game, dies_event);
        assert_eq!(triggered.len(), 1, "expected attached Aura LKI trigger");
        assert_eq!(triggered[0].source_name, "Return Aura");
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn attached_aura_lki_trigger_matches_enchanted_land_subtype() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let mountain = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Mountain])
            .build();
        let mountain_id = game.create_object_from_card(&mountain, alice, Zone::Battlefield);
        let aura_def = CardDefinitionBuilder::new(CardId::new(), "Genju Probe")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Enchant Mountain\nWhen enchanted Mountain is put into a graveyard, you may return this card from your graveyard to your hand.",
            )
            .expect("attached land graveyard trigger should parse");
        let aura = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(
                aura,
                crate::object::AttachmentTarget::Object(mountain_id),
            )
        );

        game.move_object_by_effect(mountain_id, Zone::Graveyard);
        let events = game.take_pending_trigger_events();
        let land_graveyard_event = events
            .iter()
            .find(|event| {
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|zone_change| zone_change.is_ltb())
            })
            .expect("expected land graveyard event");

        let triggered = check_triggers(&game, land_graveyard_event);
        assert_eq!(
            triggered.len(),
            1,
            "expected attached Aura LKI trigger for enchanted Mountain"
        );
        assert_eq!(triggered[0].source_name, "Genju Probe");
    }

    #[cfg(all(ironsmith_runtime_parser_tests, feature = "generated-registry"))]
    #[test]
    fn voice_of_resurgence_spell_cast_trigger_works_under_control_aura() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        let voice = crate::cards::CardRegistry::try_compile_card("Voice of Resurgence")
            .expect("Voice of Resurgence should compile");
        let treachery = crate::cards::CardRegistry::try_compile_card("Treachery")
            .expect("Treachery should compile");
        let bolt = crate::cards::CardRegistry::try_compile_card("Lightning Bolt")
            .expect("Lightning Bolt should compile");

        let voice_id = game.create_object_from_definition(&voice, bob, Zone::Battlefield);
        let treachery_id = game.create_object_from_definition(&treachery, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(
            treachery_id,
            crate::object::AttachmentTarget::Object(voice_id),
        ));
        game.refresh_continuous_state();
        assert_eq!(game.controller_of_id(voice_id), Some(alice));

        let spell_id = game.create_object_from_definition(&bolt, bob, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, bob));

        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let triggered = check_triggers(&game, &event);

        assert_eq!(triggered.len(), 1, "expected Voice spell-cast trigger");
        assert_eq!(triggered[0].controller, alice);
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
    fn temporary_next_spell_static_grant_stays_on_stack_object_after_consumption() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let spell = CardBuilder::new(CardId::from_raw(9003), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, alice));

        game.add_temporary_spell_ability_grant(
            alice,
            spell_id,
            crate::target::ObjectFilter::instant_or_sorcery().cast_by(crate::PlayerFilter::You),
            StaticAbility::cant_be_countered_ability(),
            1,
        );

        game.consume_temporary_spell_ability_grants_for_spell(spell_id, alice);

        let spell = game
            .object(spell_id)
            .expect("spell should still exist on the stack");
        assert!(
            spell.abilities.iter().any(|ability| matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::CantBeCountered
            )),
            "consumed next-spell static grant should attach to the spell object"
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
            costs: vec![("Conspire".into(), 1), ("Conspire 2".into(), 1)],
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
        let tagged = tagged_objects_for_matched_trigger(
            &game,
            &trigger_event,
            &Trigger::this_attacks_with_exact_n_others(1),
        );
        let other_attackers = tagged
            .get(&crate::tag::TagKey::from("other_attacker"))
            .expect("expected other_attacker tag for exact partner attack event");

        assert_eq!(other_attackers.len(), 1);
        assert_eq!(other_attackers[0].object_id, partner);
    }

    #[test]
    fn plain_attack_trigger_does_not_materialize_other_attackers_tag() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_creature(&mut game, alice, "Source");
        let partner = make_battlefield_creature(&mut game, alice, "Partner");
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: source,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: partner,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..Default::default()
        });
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(source, AttackEventTarget::Player(bob), 2),
            crate::provenance::ProvNodeId::default(),
        );

        let tagged = tagged_objects_for_matched_trigger(&game, &event, &Trigger::this_attacks());

        assert!(!tagged.contains_key(&crate::tag::TagKey::from("other_attacker")));
    }

    #[test]
    fn attack_event_batch_reuses_view_and_source_local_subscribers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut attackers = Vec::new();
        for index in 0..32 {
            let source =
                make_battlefield_creature(&mut game, alice, &format!("Self Attack Source {index}"));
            add_battlefield_trigger(&mut game, source, Trigger::this_attacks());
            attackers.push(source);
        }
        game.combat = Some(crate::combat_state::CombatState {
            attackers: attackers
                .iter()
                .map(|source| crate::combat_state::AttackerInfo {
                    creature: *source,
                    target: AttackTarget::Player(bob),
                })
                .collect(),
            ..Default::default()
        });
        game.refresh_continuous_state();
        let events = attackers
            .iter()
            .map(|source| {
                TriggerEvent::new_with_provenance(
                    CreatureAttackedEvent::with_total_attackers(
                        *source,
                        AttackEventTarget::Player(bob),
                        attackers.len(),
                    ),
                    crate::provenance::ProvNodeId::default(),
                )
            })
            .collect::<Vec<_>>();

        let before = game.work_counters();
        let trigger_groups = check_triggers_batch(&game, &events);
        let after = game.work_counters();

        assert_eq!(
            after.derived_view_rebuilds - before.derived_view_rebuilds,
            1
        );
        assert_eq!(trigger_groups.len(), attackers.len());
        for (expected_source, triggers) in attackers.iter().zip(trigger_groups) {
            assert_eq!(triggers.len(), 1);
            assert_eq!(triggers[0].source, *expected_source);
            assert!(
                !triggers[0]
                    .tagged_objects
                    .contains_key(&crate::tag::TagKey::from("other_attacker"))
            );
        }
    }

    #[test]
    fn dirty_registry_rebuild_batches_battlefield_characteristics() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut sources = Vec::new();
        for index in 0..32 {
            let source = make_battlefield_creature(
                &mut game,
                alice,
                &format!("Layered Attack Source {index}"),
            );
            add_battlefield_trigger(&mut game, source, Trigger::this_attacks());
            sources.push(source);
        }

        let effect_source = sources[0];
        // Keep this fixture on the dependency-sort path. An unconditional
        // add/remove pair is timestamp-only and is intentionally optimized to
        // skip baseline dependency analysis altogether.
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                effect_source,
                alice,
                crate::continuous::EffectTarget::AllPermanents,
                crate::continuous::Modification::AddAbility(StaticAbility::flying()),
            )
            .with_condition(crate::ConditionExpr::YourTurn),
        );
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                effect_source,
                alice,
                crate::continuous::EffectTarget::AllPermanents,
                crate::continuous::Modification::RemoveAbility(StaticAbility::flying()),
            ));
        game.refresh_continuous_state();

        // Mana payment and similar action plumbing can invalidate continuous
        // state without changing the effect list. Registry construction must
        // still use the layer batch rather than one full pass per permanent.
        game.mark_continuous_state_dirty();
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                effect_source,
                AttackEventTarget::Player(bob),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let before = game.work_counters();

        let triggered = check_triggers(&game, &event);

        let after = game.work_counters();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source, effect_source);
        assert_eq!(
            after.dependency_sorts - before.dependency_sorts,
            1,
            "a dirty registry rebuild should sort the shared layer batch once"
        );
    }

    #[test]
    fn trigger_registry_rebuilds_when_combat_activates_granted_trigger() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_creature(&mut game, alice, "Conditional Attack Source");
        add_conditional_battlefield_trigger_grant(
            &mut game,
            source,
            alice,
            crate::ConditionExpr::SourceIsAttacking,
        );
        game.refresh_continuous_state();

        let event = || {
            TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::with_total_attackers(
                    source,
                    AttackEventTarget::Player(bob),
                    1,
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };

        assert!(
            check_triggers(&game, &event()).is_empty(),
            "the conditional trigger should not be registered before its source attacks"
        );

        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..Default::default()
        });
        game.mark_continuous_state_dirty();
        game.refresh_continuous_state();

        let triggered = check_triggers(&game, &event());
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source, source);
    }

    #[test]
    fn trigger_registry_rebuilds_when_turn_context_activates_granted_trigger() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = make_battlefield_creature(&mut game, alice, "Your Turn Attack Source");
        add_conditional_battlefield_trigger_grant(
            &mut game,
            source,
            alice,
            crate::ConditionExpr::YourTurn,
        );
        game.turn.active_player = bob;
        game.refresh_continuous_state();

        let event = || {
            TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::with_total_attackers(
                    source,
                    AttackEventTarget::Player(bob),
                    1,
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };

        assert!(
            check_triggers(&game, &event()).is_empty(),
            "the conditional trigger should not be registered during another player's turn"
        );

        // Turn progression changes these fields directly. It does not need an
        // object or effect-list mutation to change a conditional grant.
        game.turn.active_player = alice;

        let triggered = check_triggers(&game, &event());
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source, source);
    }

    #[test]
    fn becomes_targeted_event_captures_targeting_stack_object_tag() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let target = make_battlefield_creature(&mut game, alice, "Protected Creature");
        let source = make_battlefield_creature(&mut game, alice, "Targeting Ability Source");

        let trigger_event = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(target, source, alice, true),
            crate::provenance::ProvNodeId::default(),
        );
        let tagged = tagged_objects_for_trigger_event(&game, &trigger_event);
        let source_tag = tagged
            .get(&crate::tag::TagKey::from("triggering_source"))
            .expect("becomes-targeted events should expose the targeting source");

        assert_eq!(source_tag.len(), 1);
        assert_eq!(source_tag[0].object_id, source);
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

    #[cfg(ironsmith_runtime_parser_tests)]
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

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn living_artifact_player_damage_trigger_matches_only_controller_damage() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let def = CardDefinitionBuilder::new(CardId::new(), "Living Artifact Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Whenever you're dealt damage, put that many vitality counters on this Aura.",
            )
            .expect("Living Artifact style trigger should parse");
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let to_controller = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                DamageEvent::with_cause(
                    source,
                    DamageTarget::Player(alice),
                    3,
                    false,
                    EventCause::from_game_rule(),
                ),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert_eq!(
            to_controller.len(),
            1,
            "expected trigger on controller damage"
        );

        let to_opponent = check_triggers(
            &game,
            &TriggerEvent::new_with_provenance(
                DamageEvent::with_cause(
                    source,
                    DamageTarget::Player(bob),
                    3,
                    false,
                    EventCause::from_game_rule(),
                ),
                crate::provenance::ProvNodeId::default(),
            ),
        );
        assert!(
            to_opponent.is_empty(),
            "trigger should not fire when a different player is dealt damage"
        );
    }
}
