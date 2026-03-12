//! Trigger checking and queue management.
//!
//! This module contains the `check_triggers()` function that scans all permanents
//! for triggered abilities that match a game event.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::Effect;
use crate::ability::{AbilityKind, TriggeredAbility};
use crate::continuous::ContinuousEffect;
use crate::filter::ObjectRef;
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;
use crate::types::CardType;
use crate::zone::Zone;

use super::Trigger;
use super::TriggerEvent;
use super::matcher_trait::TriggerContext;

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
    pub effects: Vec<Effect>,
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
    trigger_ability.effects.len().hash(&mut hasher);
    trigger_ability.choices.len().hash(&mut hasher);
    trigger_ability.intervening_if.is_some().hash(&mut hasher);
    for effect in &trigger_ability.effects {
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
    delayed.effects.len().hash(&mut hasher);
    delayed.one_shot.hash(&mut hasher);
    delayed.not_before_turn.hash(&mut hasher);
    delayed.expires_at_turn.hash(&mut hasher);
    delayed.controller.hash(&mut hasher);
    for effect in &delayed.effects {
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

fn monarch_designation_source() -> (ObjectId, StableId, String) {
    let source = ObjectId::from_raw(0);
    (source, StableId::from(source), "The Monarch".to_string())
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
                effects: vec![Effect::target_draws(1, PlayerFilter::Specific(monarch))],
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
                effects: vec![Effect::become_monarch_player(PlayerFilter::Specific(
                    source_obj.controller,
                ))],
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
            .abilities(obj_id)
            .unwrap_or_else(|| obj.abilities.clone());

        // Check each ability on the permanent
        for ability in &calculated_abilities {
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
                    x_value: obj.x_value,
                    ability: TriggeredAbility {
                        trigger: trigger_ability.trigger.clone(),
                        effects: trigger_ability.effects.clone(),
                        choices: trigger_ability.choices.clone(),
                        intervening_if: trigger_ability.intervening_if.clone(),
                    },
                    triggering_event: trigger_event.clone(),
                    source_stable_id: obj.stable_id,
                    source_name: obj.name.clone(),
                    tagged_objects: std::collections::HashMap::new(),
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
                        x_value: snapshot.x_value,
                        ability: TriggeredAbility {
                            trigger: trigger_ability.trigger.clone(),
                            effects: trigger_ability.effects.clone(),
                            choices: trigger_ability.choices.clone(),
                            intervening_if: trigger_ability.intervening_if.clone(),
                        },
                        triggering_event: trigger_event.clone(),
                        source_stable_id: snapshot.stable_id,
                        source_name: snapshot.name.clone(),
                        tagged_objects: std::collections::HashMap::new(),
                        trigger_identity,
                    };
                    for _ in 0..trigger_count {
                        triggered.push(entry.clone());
                    }
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

    // Check spell objects on the stack for abilities like
    // "When you cast this spell".
    for entry in &game.stack {
        let Some(obj) = game.object(entry.object_id) else {
            continue;
        };
        if obj.zone != Zone::Stack {
            continue;
        }
        check_triggers_in_zone(game, obj.id, trigger_event, &mut triggered);
    }

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
        let cascade_count = obj
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
        if cascade_count > 0 {
            let ability = TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: vec![Effect::new(crate::effects::CascadeEffect::new())],
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
                    tagged_objects: std::collections::HashMap::new(),
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
                effects,
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
                tagged_objects: std::collections::HashMap::new(),
                trigger_identity,
            });
        }
    }

    add_monarch_designation_triggers(game, trigger_event, &mut triggered);

    triggered
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
                tagged_objects: delayed.tagged_objects.clone(),
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
                x_value: obj.x_value,
                ability: TriggeredAbility {
                    trigger: trigger_ability.trigger.clone(),
                    effects: trigger_ability.effects.clone(),
                    choices: trigger_ability.choices.clone(),
                    intervening_if: trigger_ability.intervening_if.clone(),
                },
                triggering_event: trigger_event.clone(),
                source_stable_id: obj.stable_id,
                source_name: obj.name.clone(),
                tagged_objects: std::collections::HashMap::new(),
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
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source: source_object_id,
        defending_player: None,
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
