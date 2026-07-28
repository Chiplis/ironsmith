//! Per-blocking-pair triggers with a relative power qualification.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

fn matches_event_object(
    filter: &ObjectFilter,
    snapshot: Option<&ObjectSnapshot>,
    object_id: ObjectId,
    ctx: &TriggerContext<'_>,
) -> bool {
    snapshot.map_or_else(
        || {
            ctx.game
                .object(object_id)
                .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
        },
        |snapshot| filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game),
    )
}

fn event_power(
    snapshot: Option<&ObjectSnapshot>,
    object_id: ObjectId,
    ctx: &TriggerContext<'_>,
) -> Option<i32> {
    snapshot
        .and_then(|snapshot| snapshot.power)
        .or_else(|| ctx.game.calculated_power(object_id))
        .or_else(|| ctx.game.object(object_id).and_then(|object| object.power()))
}

fn with_indefinite_article(description: String) -> String {
    let trimmed = description.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("another ")
        || lower.starts_with("the ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("each ")
        || lower.starts_with("one or more ")
    {
        return trimmed.to_string();
    }
    let article = if trimmed
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

/// "Whenever [blocker] blocks [blocked object] with lesser power."
#[derive(Debug, Clone, PartialEq)]
pub struct BlocksObjectWithLesserPowerTrigger {
    pub blocker: ObjectFilter,
    pub blocked: ObjectFilter,
}

impl BlocksObjectWithLesserPowerTrigger {
    pub fn new(blocker: ObjectFilter, blocked: ObjectFilter) -> Self {
        Self { blocker, blocked }
    }
}

impl TriggerMatcher for BlocksObjectWithLesserPowerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext<'_>) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(event) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if !matches_event_object(
            &self.blocker,
            event.blocker_snapshot.as_ref(),
            event.blocker,
            ctx,
        ) || !matches_event_object(
            &self.blocked,
            event.attacker_snapshot.as_ref(),
            event.attacker,
            ctx,
        ) {
            return false;
        }

        event_power(event.attacker_snapshot.as_ref(), event.attacker, ctx)
            .zip(event_power(
                event.blocker_snapshot.as_ref(),
                event.blocker,
                ctx,
            ))
            .is_some_and(|(blocked_power, blocker_power)| blocked_power < blocker_power)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureBlocked])
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} blocks {} with lesser power",
            with_indefinite_article(self.blocker.description()),
            with_indefinite_article(self.blocked.description())
        )
    }
}

/// "Whenever [blocked object] becomes blocked by [blocker] with lesser power."
#[derive(Debug, Clone, PartialEq)]
pub struct BecomesBlockedByObjectWithLesserPowerTrigger {
    pub blocked: ObjectFilter,
    pub blocker: ObjectFilter,
}

impl BecomesBlockedByObjectWithLesserPowerTrigger {
    pub fn new(blocked: ObjectFilter, blocker: ObjectFilter) -> Self {
        Self { blocked, blocker }
    }
}

impl TriggerMatcher for BecomesBlockedByObjectWithLesserPowerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext<'_>) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(event) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if !matches_event_object(
            &self.blocked,
            event.attacker_snapshot.as_ref(),
            event.attacker,
            ctx,
        ) || !matches_event_object(
            &self.blocker,
            event.blocker_snapshot.as_ref(),
            event.blocker,
            ctx,
        ) {
            return false;
        }

        event_power(event.blocker_snapshot.as_ref(), event.blocker, ctx)
            .zip(event_power(
                event.attacker_snapshot.as_ref(),
                event.attacker,
                ctx,
            ))
            .is_some_and(|(blocker_power, blocked_power)| blocker_power < blocked_power)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureBlocked])
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} becomes blocked by {} with lesser power",
            with_indefinite_article(self.blocked.description()),
            with_indefinite_article(self.blocker.description())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, power))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn block_event(game: &GameState, blocker: ObjectId, attacker: ObjectId) -> TriggerEvent {
        let blocker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(blocker).expect("blocker"),
            game,
        );
        let attacker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).expect("attacker"),
            game,
        );
        TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::with_snapshots(
                blocker,
                attacker,
                blocker_snapshot,
                attacker_snapshot,
            ),
            ProvNodeId::default(),
        )
    }

    #[test]
    fn compares_the_two_block_participants_in_the_authored_direction() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", alice, 5);
        let blocker = create_creature(&mut game, "Blocker", bob, 2);
        let event = block_event(&game, blocker, attacker);
        let ctx = TriggerContext::for_source(attacker, alice, &game);

        assert!(
            BecomesBlockedByObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .matches(&event, &ctx)
        );
        assert!(
            !BlocksObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .matches(&event, &ctx)
        );
    }

    #[test]
    fn equal_power_is_not_lesser_power() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", alice, 3);
        let blocker = create_creature(&mut game, "Blocker", bob, 3);
        let event = block_event(&game, blocker, attacker);
        let ctx = TriggerContext::for_source(attacker, alice, &game);

        assert!(
            !BecomesBlockedByObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .matches(&event, &ctx)
        );
        assert!(
            !BlocksObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .matches(&event, &ctx)
        );
    }

    #[test]
    fn displays_both_relative_power_surfaces_with_articles() {
        assert_eq!(
            BlocksObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .display(),
            "Whenever a creature blocks a creature with lesser power"
        );
        assert_eq!(
            BecomesBlockedByObjectWithLesserPowerTrigger::new(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
            )
            .display(),
            "Whenever a creature becomes blocked by a creature with lesser power"
        );
    }
}
