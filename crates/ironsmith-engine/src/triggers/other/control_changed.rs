use crate::events::zones::ObjectLeavesGameEvent;
use crate::events::{ControlChangedEvent, EventKind, ZoneChangeEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::snapshot::ObjectSnapshot;
use crate::target::PlayerFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
use crate::zone::Zone;
use ironsmith_core::trigger_model::ControlChangeDirection;

pub type ControlChangedTrigger = ironsmith_core::trigger_model::ControlChangeTrigger;

fn looks_back(change: &ControlChangeDirection) -> bool {
    // CR 603.10d names both loss of control and an opponent gaining an
    // object from the ability's controller. A normal gain checks afterward.
    matches!(
        change,
        ControlChangeDirection::Lost { .. }
            | ControlChangeDirection::Gained {
                player: PlayerFilter::Opponent,
                from: Some(PlayerFilter::You)
            }
    )
}
fn matches_object(
    trigger: &ControlChangedTrigger,
    snapshot: &ObjectSnapshot,
    ctx: &TriggerContext,
) -> bool {
    snapshot.zone == Zone::Battlefield
        && trigger
            .filter
            .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
}
fn loss_snapshots<'a>(event: &'a TriggerEvent) -> Vec<&'a ObjectSnapshot> {
    if let Some(zone) = event.downcast::<ZoneChangeEvent>() {
        if zone.from == Zone::Battlefield && zone.to != Zone::Battlefield {
            return zone.snapshots().iter().collect();
        }
    }
    if let Some(left) = event.downcast::<ObjectLeavesGameEvent>() {
        if left.snapshot.zone == Zone::Battlefield {
            return vec![&left.snapshot];
        }
    }
    Vec::new()
}
impl TriggerMatcher for ControlChangedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if let Some(control) = event.downcast::<ControlChangedEvent>() {
            if control.previous_controller == control.new_controller {
                return false;
            }
            let players_match = match &self.change {
                ControlChangeDirection::Gained { player, from } => {
                    player.matches_player(control.new_controller, &ctx.filter_ctx)
                        && from.as_ref().is_none_or(|from| {
                            from.matches_player(control.previous_controller, &ctx.filter_ctx)
                        })
                }
                ControlChangeDirection::Lost { player } => {
                    player.matches_player(control.previous_controller, &ctx.filter_ctx)
                }
            };
            let snapshot = if looks_back(&self.change) {
                control.previous_snapshot.as_ref()
            } else {
                control.snapshot.as_ref()
            };
            return players_match
                && snapshot.is_some_and(|snapshot| {
                    snapshot.object_id == control.permanent && matches_object(self, snapshot, ctx)
                });
        }
        let ControlChangeDirection::Lost { player } = &self.change else {
            return false;
        };
        loss_snapshots(event).into_iter().any(|snapshot| {
            player.matches_player(snapshot.controller, &ctx.filter_ctx)
                && matches_object(self, snapshot, ctx)
        })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        let mut kinds = vec![EventKind::ControlChanged];
        if matches!(&self.change, ControlChangeDirection::Lost { .. }) {
            kinds.extend([EventKind::ZoneChange, EventKind::ObjectLeavesGame]);
        }
        Some(kinds)
    }
    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        looks_back(&self.change)
            && self
                .subscribed_kinds()
                .is_some_and(|kinds| kinds.contains(&event.kind()))
    }
    fn uses_snapshot(&self) -> bool {
        true
    }
    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if event.kind() == EventKind::ControlChanged {
            return u32::from(self.matches(event, ctx));
        }
        let ControlChangeDirection::Lost { player } = &self.change else {
            return 0;
        };
        loss_snapshots(event)
            .into_iter()
            .filter(|snapshot| {
                player.matches_player(snapshot.controller, &ctx.filter_ctx)
                    && matches_object(self, snapshot, ctx)
            })
            .count() as u32
    }
    fn display(&self) -> String {
        let (player, gained, from) = match &self.change {
            ControlChangeDirection::Gained { player, from } => (player, true, from.as_ref()),
            ControlChangeDirection::Lost { player } => (player, false, None),
        };
        let verb = match (gained, player == &PlayerFilter::You) {
            (true, true) => "gain",
            (true, false) => "gains",
            (false, true) => "lose",
            (false, false) => "loses",
        };
        let from = from
            .map(|player| {
                format!(
                    " from {}",
                    if player == &PlayerFilter::NotYou {
                        "another player".into()
                    } else {
                        player.description()
                    }
                )
            })
            .unwrap_or_default();
        format!(
            "When {} {verb} control of {}{from}",
            player.description(),
            self.filter.description()
        )
    }
}
