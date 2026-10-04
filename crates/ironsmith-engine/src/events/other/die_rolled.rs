//! Die roll event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A player rolled a die and got a result.
#[derive(Debug, Clone)]
pub struct DieRolledEvent {
    pub player: PlayerId,
    pub source: ObjectId,
    pub natural_result: u32,
    pub result: u32,
    pub sides: u32,
    /// Planar-die rolls trigger generic roll observers but have no numerical
    /// result for effects that compare or inspect die numbers (CR 901.9d).
    pub is_planar: bool,
    /// True only for a roll made to visit Attractions, including rolls caused
    /// by an effect. An ordinary d6 roll is not an Attraction visit roll.
    pub is_attraction_visit: bool,
    /// Ordinal at the actual completed roll, never reconstructed at resolution.
    pub ordinal_this_turn: Option<u32>,
    pub(crate) batch_results: Option<std::sync::Arc<DieRollBatchResults>>,
}

#[derive(Debug, Clone)]
pub(crate) struct DieRollBatchResults {
    pub results: Vec<u32>,
}

impl DieRolledEvent {
    pub fn new(player: PlayerId, source: ObjectId, result: u32, sides: u32) -> Self {
        Self::new_with_natural_result(player, source, result, result, sides)
    }

    pub fn new_with_natural_result(
        player: PlayerId,
        source: ObjectId,
        natural_result: u32,
        result: u32,
        sides: u32,
    ) -> Self {
        Self {
            player,
            source,
            natural_result,
            result,
            sides,
            is_planar: false,
            is_attraction_visit: false,
            ordinal_this_turn: None,
            batch_results: None,
        }
    }

    pub fn new_planar(player: PlayerId, source: ObjectId, encoded_face: u32) -> Self {
        Self {
            player,
            source,
            natural_result: encoded_face,
            result: encoded_face,
            sides: 6,
            is_planar: true,
            is_attraction_visit: false,
            ordinal_this_turn: None,
            batch_results: None,
        }
    }

    pub fn with_turn_ordinal(mut self, ordinal: u32) -> Self {
        self.ordinal_this_turn = Some(ordinal);
        self
    }
    pub(crate) fn numeric_batch_results(&self) -> impl Iterator<Item = u32> + '_ {
        let own = (!self.is_planar && self.batch_results.is_none()).then_some(self.result);
        self.batch_results
            .iter()
            .flat_map(|batch| batch.results.iter().copied())
            .chain(own)
    }

    pub fn for_attraction_visit(mut self) -> Self {
        self.is_attraction_visit = true;
        self
    }
}

impl GameEventType for DieRolledEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DieRolled
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn display(&self) -> String {
        if self.is_planar {
            "Player rolled the planar die".to_string()
        } else {
            format!("Player rolled a {} on a d{}", self.result, self.sides)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Each player's dice in one simultaneous action are one grouped occurrence.
/// Physical dice remain separate events for individual-result triggers.
pub(crate) fn bind_die_roll_batch_results(events: &mut [crate::triggers::TriggerEvent]) {
    let mut groups: std::collections::HashMap<PlayerId, Vec<u32>> =
        std::collections::HashMap::new();
    for event in events.iter() {
        if let Some(roll) = event.downcast::<DieRolledEvent>() {
            let group = groups.entry(roll.player).or_default();
            if !roll.is_planar {
                group.push(roll.result);
            }
        }
    }
    let groups: std::collections::HashMap<_, _> = groups
        .into_iter()
        .map(|(player, results)| (player, std::sync::Arc::new(DieRollBatchResults { results })))
        .collect();
    for event in events {
        if let Some(roll) = event.downcast::<DieRolledEvent>() {
            let mut roll = roll.clone();
            roll.batch_results = Some(groups[&roll.player].clone());
            *event = event.with_inner_event(roll);
        }
    }
}

#[cfg(test)]
mod receipt_tests {
    use super::*;
    use crate::triggers::TriggerEvent;
    #[test]
    fn batch_results_preserve_players_and_exclude_nonnumeric_faces() {
        let source = ObjectId::from_raw(1);
        let event = |player, result| {
            TriggerEvent::new_with_provenance(
                DieRolledEvent::new(PlayerId(player), source, result, 20),
                Default::default(),
            )
        };
        let mut events = vec![
            event(0, 2),
            event(0, 12),
            event(1, 7),
            TriggerEvent::new_with_provenance(
                DieRolledEvent::new_planar(PlayerId(0), source, 6),
                Default::default(),
            ),
        ];
        bind_die_roll_batch_results(&mut events);
        assert_eq!(
            events[0]
                .downcast::<DieRolledEvent>()
                .unwrap()
                .numeric_batch_results()
                .collect::<Vec<_>>(),
            vec![2, 12]
        );
        assert_eq!(
            events[2]
                .downcast::<DieRolledEvent>()
                .unwrap()
                .numeric_batch_results()
                .collect::<Vec<_>>(),
            vec![7]
        );
        let game = crate::GameState::new(vec!["A".into(), "B".into()], 30);
        let ctx = crate::effects::ExecutionContext::new_default(source, PlayerId(0))
            .with_triggering_event(events[0].clone());
        assert_eq!(
            crate::effects::helpers::resolve_value(
                &game,
                &crate::effect::Value::EventValue(crate::effect::EventValueSpec::DieBatchTotal),
                &ctx
            )
            .unwrap(),
            14
        );
        assert_eq!(
            crate::effects::helpers::resolve_value(
                &game,
                &crate::effect::Value::EventValue(
                    crate::effect::EventValueSpec::DieResultsAtLeast(10)
                ),
                &ctx
            )
            .unwrap(),
            1
        );
    }
}
