use crate::events::{EventKind, PermanentTappedEvent, PermanentUntappedEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerChangesTapStateTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub tapped: bool,
    pub one_or_more: bool,
    pub during_untap_step: Option<PlayerFilter>,
}

impl TriggerMatcher for PlayerChangesTapStateTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let (actor, before, step_players) = if self.tapped {
            let Some(event) = event.downcast::<PermanentTappedEvent>() else {
                return false;
            };
            (event.actor, event.before_snapshot.as_ref(), &[][..])
        } else {
            let Some(event) = event.downcast::<PermanentUntappedEvent>() else {
                return false;
            };
            (
                event.actor,
                event.before_snapshot.as_ref(),
                event.untap_step_players.as_slice(),
            )
        };
        actor.is_some_and(|actor| self.player.matches_player(actor, &ctx.filter_ctx))
            && self.during_untap_step.as_ref().is_none_or(|player| {
                step_players
                    .iter()
                    .any(|id| player.matches_player(*id, &ctx.filter_ctx))
            })
            && before.is_some_and(|snapshot| {
                self.filter
                    .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
            })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![if self.tapped {
            EventKind::PermanentTapped
        } else {
            EventKind::PermanentUntapped
        }])
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        let kind = if self.tapped {
            EventKind::PermanentTapped
        } else {
            EventKind::PermanentUntapped
        };
        if !self.one_or_more || event.kind() != kind {
            return None;
        }
        let actor = if self.tapped {
            event.downcast::<PermanentTappedEvent>()?.actor?
        } else {
            event.downcast::<PermanentUntappedEvent>()?.actor?
        };
        Some(SimultaneousTriggerKey::PlayerTapStateBatch {
            tapped: self.tapped,
            actor,
        })
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.matches(event, ctx).then_some(1)
    }

    fn display(&self) -> String {
        let mut filter = self.filter.clone();
        filter.set_plural_object_noun_surface(self.one_or_more);
        let verb = if self.tapped { "tap" } else { "untap" };
        let player = crate::triggers::describe_player_filter_subject(&self.player);
        let agreement = if self.player == PlayerFilter::You {
            ""
        } else {
            "s"
        };
        let description = filter.description();
        let quantifier = if self.one_or_more {
            "one or more "
        } else if description.starts_with(['a', 'e', 'i', 'o', 'u']) {
            "an "
        } else {
            "a "
        };
        let step = self
            .during_untap_step
            .as_ref()
            .map(|player| match player {
                PlayerFilter::You => " during your untap step".to_owned(),
                _ => format!(
                    " during {}'s untap step",
                    crate::triggers::describe_player_filter_subject(player)
                ),
            })
            .unwrap_or_default();
        format!(
            "Whenever {player} {verb}{agreement} {quantifier}{}{step}",
            description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn grouped_active_events_preserve_distinct_authored_player_subjects() {
        let trigger = PlayerChangesTapStateTrigger {
            player: PlayerFilter::Any,
            filter: ObjectFilter::permanent(),
            tapped: false,
            one_or_more: true,
            during_untap_step: None,
        };
        let first = TriggerEvent::new(
            PermanentUntappedEvent::new(ObjectId::from_raw(1)).with_actor(PlayerId(0)),
            Default::default(),
        );
        let second = TriggerEvent::new(
            PermanentUntappedEvent::new(ObjectId::from_raw(2)).with_actor(PlayerId(1)),
            Default::default(),
        );
        assert_ne!(
            trigger.simultaneous_trigger_key(&first),
            trigger.simultaneous_trigger_key(&second)
        );
        let third = TriggerEvent::new(
            PermanentUntappedEvent::new(ObjectId::from_raw(3)).with_actor(PlayerId(0)),
            Default::default(),
        );
        assert_eq!(
            trigger.simultaneous_trigger_key(&first),
            trigger.simultaneous_trigger_key(&third)
        );
    }
}
