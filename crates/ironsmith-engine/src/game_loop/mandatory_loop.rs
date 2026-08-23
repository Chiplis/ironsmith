use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::events::EventKind;
use crate::game_state::GameState;
use crate::game_state::StackEntry;
use crate::ids::{PlayerId, StableId};
use crate::triggers::{TriggerIdentity, TriggeredAbilityEntry};

const MAX_MANDATORY_ACTION_HISTORY: usize = 64;

/// Structural identity for one automatically repeating rules procedure.
///
/// Provenance nodes and transient stack object IDs are deliberately excluded:
/// both change on every trip through a loop. The source's stable identity,
/// controller, procedure structure, triggering-event surface, and control
/// fingerprint are the rules-relevant identity of the action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MandatoryProcedureKind {
    Triggered {
        trigger: TriggerIdentity,
        event_kind: EventKind,
        event_surface: String,
    },
    StackResolution {
        is_ability: bool,
        ability_index: Option<usize>,
        program_surface: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MandatoryProcedureSignature {
    source: StableId,
    controller: PlayerId,
    kind: MandatoryProcedureKind,
    control_fingerprint: u64,
}

#[derive(Debug, Clone)]
pub(super) struct MandatoryProcedureObservation {
    signature: MandatoryProcedureSignature,
    blocks_mandatory_proof: bool,
}

impl MandatoryProcedureObservation {
    pub(super) fn from_stack_entry(game: &GameState, entry: &StackEntry) -> Option<Self> {
        let source = entry
            .source_stable_id
            .or_else(|| game.object(entry.object_id).map(|object| object.stable_id))
            .or_else(|| {
                entry
                    .source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.stable_id)
            })?;
        let kind = match (entry.trigger_identity, entry.triggering_event.as_ref()) {
            (Some(trigger), Some(event)) => MandatoryProcedureKind::Triggered {
                trigger,
                event_kind: event.kind(),
                event_surface: event.display(),
            },
            _ => MandatoryProcedureKind::StackResolution {
                is_ability: entry.is_ability,
                ability_index: entry.ability_index,
                program_surface: format!("{:?}", entry.ability_effects),
            },
        };
        let event_kind = entry.triggering_event.as_ref().map(|event| event.kind());
        Some(Self {
            signature: MandatoryProcedureSignature {
                source,
                controller: entry.controller,
                kind,
                control_fingerprint: source_control_fingerprint(
                    game,
                    source,
                    entry.source_snapshot.as_ref(),
                    event_kind,
                ),
            },
            blocks_mandatory_proof: entry.intervening_if.is_some()
                || !entry.target_assignments.is_empty()
                || entry
                    .ability_effects
                    .as_ref()
                    .is_some_and(program_contains_control_branch),
        })
    }

    pub(super) fn from_trigger_entry(game: &GameState, entry: &TriggeredAbilityEntry) -> Self {
        Self {
            signature: MandatoryProcedureSignature {
                source: entry.source_stable_id,
                controller: entry.controller,
                kind: MandatoryProcedureKind::Triggered {
                    trigger: entry.trigger_identity,
                    event_kind: entry.triggering_event.kind(),
                    event_surface: entry.triggering_event.display(),
                },
                control_fingerprint: source_control_fingerprint(
                    game,
                    entry.source_stable_id,
                    entry.source_snapshot.as_ref(),
                    Some(entry.triggering_event.kind()),
                ),
            },
            blocks_mandatory_proof: entry.ability.intervening_if.is_some()
                || !entry.ability.choices.is_empty()
                || program_contains_control_branch(&entry.ability.effects),
        }
    }
}

fn program_contains_control_branch(program: &crate::resolution::ResolutionProgram) -> bool {
    program
        .all_effects()
        .into_iter()
        .any(effect_contains_control_branch)
}

fn effect_contains_control_branch(effect: &crate::effect::Effect) -> bool {
    if effect.downcast_ref::<crate::effects::MayEffect>().is_some()
        || effect
            .downcast_ref::<crate::effects::ChooseModeEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::UnlessActionEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::UnlessPaysEffect>()
            .is_some()
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && effect_contains_control_branch(child) {
            found = true;
        }
    });
    found
}

fn source_control_fingerprint(
    game: &GameState,
    source: StableId,
    fallback: Option<&crate::snapshot::ObjectSnapshot>,
    repeating_event: Option<EventKind>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    for player in &game.players {
        let mut player = player.clone();
        if matches!(
            repeating_event,
            Some(EventKind::LifeGain | EventKind::LifeLoss)
        ) {
            // The repeating life event may be unbounded without influencing
            // whether the same procedure remains mandatory. Preserve every
            // other player field so finite resource/control changes still
            // produce a different fingerprint.
            player.life = 0;
        }
        if matches!(repeating_event, Some(EventKind::ManaAdded)) {
            // A triggered mana procedure can add the same unit forever. The
            // growing pool is the repeated event's payload, not an exit from
            // the procedure; preserve all non-mana resources and object state.
            player.mana_pool = crate::player::ManaPool::default();
            player.restricted_mana.clear();
            player.mana_source_provenance.clear();
        }
        format!("{player:?}").hash(&mut hasher);
    }
    format!("{:?}", game.turn).hash(&mut hasher);
    for entry in game.stack.iter().take(game.stack.len().saturating_sub(1)) {
        format!("{entry:?}").hash(&mut hasher);
    }
    for object_id in game.object_ids_in_deterministic_order() {
        if let Some(object) = game.object(object_id) {
            format!("{object:?}").hash(&mut hasher);
        }
    }
    if game.find_object_by_stable_id(source).is_none() {
        if let Some(snapshot) = fallback {
            format!("{snapshot:?}").hash(&mut hasher);
        } else {
            source.hash(&mut hasher);
        }
    }
    hasher.finish()
}

impl MandatoryProcedureSignature {
    #[cfg(test)]
    fn triggered(source: StableId, controller: PlayerId, trigger: TriggerIdentity) -> Self {
        Self {
            source,
            controller,
            kind: MandatoryProcedureKind::Triggered {
                trigger,
                event_kind: EventKind::LifeGain,
                event_surface: "Gain 1 life".to_string(),
            },
            control_fingerprint: 0,
        }
    }
}

/// Tracks the mandatory rules-procedure sequence within one priority epoch.
///
/// Any player-visible alternative action invalidates the candidate sequence.
/// A draw is reported only after an action suffix has actually repeated and
/// the trigger queue contains the action that would begin the same suffix
/// again. This prevents a completed finite sequence from being mistaken for a
/// loop merely because its final two actions happened to share an identity.
#[derive(Debug, Clone, Default)]
pub(super) struct MandatoryLoopTracker {
    actions: Vec<MandatoryProcedureSignature>,
    optional_action_seen: bool,
}

impl MandatoryLoopTracker {
    pub(super) fn reset(&mut self) {
        self.actions.clear();
        self.optional_action_seen = false;
    }

    pub(super) fn observe_priority_window(&mut self, forced_pass: bool) {
        if !forced_pass {
            self.optional_action_seen = true;
        }
    }

    pub(super) fn observe_player_action(&mut self) {
        self.reset();
        self.optional_action_seen = true;
    }

    pub(super) fn observe_resolution(
        &mut self,
        resolved: Option<MandatoryProcedureObservation>,
        queued: impl IntoIterator<Item = MandatoryProcedureObservation>,
    ) -> Option<std::collections::HashSet<PlayerId>> {
        if self.optional_action_seen {
            self.reset();
            return None;
        }
        let Some(resolved) = resolved else {
            self.reset();
            return None;
        };
        if resolved.blocks_mandatory_proof {
            self.reset();
            return None;
        }

        self.actions.push(resolved.signature);
        if self.actions.len() > MAX_MANDATORY_ACTION_HISTORY {
            let overflow = self.actions.len() - MAX_MANDATORY_ACTION_HISTORY;
            self.actions.drain(..overflow);
        }

        let Some((next_expected, controllers)) = self.repeated_suffix_next_action() else {
            return None;
        };
        queued
            .into_iter()
            .any(|candidate| {
                !candidate.blocks_mandatory_proof && candidate.signature == next_expected
            })
            .then_some(controllers)
    }

    fn repeated_suffix_next_action(
        &self,
    ) -> Option<(
        MandatoryProcedureSignature,
        std::collections::HashSet<PlayerId>,
    )> {
        let len = self.actions.len();
        for cycle_len in 1..=len / 2 {
            let previous_start = len - cycle_len * 2;
            let current_start = len - cycle_len;
            if self.actions[previous_start..current_start] == self.actions[current_start..] {
                let controllers = self.actions[current_start..]
                    .iter()
                    .map(|signature| signature.controller)
                    .collect();
                return self
                    .actions
                    .get(current_start)
                    .cloned()
                    .map(|next| (next, controllers));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(source: u64, trigger: u64) -> MandatoryProcedureObservation {
        observation_for(source, 0, trigger)
    }

    fn observation_for(source: u64, controller: u8, trigger: u64) -> MandatoryProcedureObservation {
        MandatoryProcedureObservation {
            signature: MandatoryProcedureSignature::triggered(
                StableId::from(crate::ids::ObjectId::from_raw(source)),
                PlayerId::from_index(controller),
                TriggerIdentity(trigger),
            ),
            blocks_mandatory_proof: false,
        }
    }

    #[test]
    fn repeated_forced_cycle_requires_the_next_action_to_be_queued() {
        let a = observation(1, 11);
        let b = observation(2, 22);
        let mut tracker = MandatoryLoopTracker::default();

        assert!(
            tracker
                .observe_resolution(Some(a.clone()), [b.clone()])
                .is_none()
        );
        assert!(
            tracker
                .observe_resolution(Some(b.clone()), [a.clone()])
                .is_none()
        );
        assert!(
            tracker
                .observe_resolution(Some(a.clone()), [b.clone()])
                .is_none()
        );
        assert!(tracker.observe_resolution(Some(b), [a]).is_some());
    }

    #[test]
    fn optional_action_breaks_the_mandatory_cycle_candidate() {
        let action = observation(1, 11);
        let mut tracker = MandatoryLoopTracker::default();

        assert!(
            tracker
                .observe_resolution(Some(action.clone()), [action.clone()])
                .is_none()
        );
        tracker.observe_priority_window(false);
        assert!(
            tracker
                .observe_resolution(Some(action.clone()), [action.clone()])
                .is_none()
        );
        assert!(
            tracker
                .observe_resolution(Some(action.clone()), [action.clone()])
                .is_none()
        );
        tracker.observe_priority_window(false);
        assert!(
            tracker
                .observe_resolution(Some(action.clone()), [action])
                .is_none()
        );
    }

    #[test]
    fn repeated_cycle_reports_every_involved_object_controller() {
        let a = observation_for(1, 0, 11);
        let b = observation_for(2, 2, 22);
        let mut tracker = MandatoryLoopTracker::default();

        assert!(
            tracker
                .observe_resolution(Some(a.clone()), [b.clone()])
                .is_none()
        );
        assert!(
            tracker
                .observe_resolution(Some(b.clone()), [a.clone()])
                .is_none()
        );
        assert!(
            tracker
                .observe_resolution(Some(a.clone()), [b.clone()])
                .is_none()
        );
        let controllers = tracker
            .observe_resolution(Some(b), [a])
            .expect("the repeated mandatory suffix should be proved");
        assert_eq!(
            controllers,
            std::collections::HashSet::from([PlayerId::from_index(0), PlayerId::from_index(2),])
        );
    }
}
