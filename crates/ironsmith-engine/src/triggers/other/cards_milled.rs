use crate::events::{CardMilledEvent, EventKind};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::matcher_trait::SimultaneousTriggerKey;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CardsMilledTrigger {
    pub player: PlayerFilter,
    pub filter: Option<ObjectFilter>,
    pub one_or_more: bool,
    pub per_player: bool,
}
impl TriggerMatcher for CardsMilledTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(event) = event.downcast::<CardMilledEvent>() else {
            return false;
        };
        self.player.matches_player(event.player, &ctx.filter_ctx)
            && self.filter.as_ref().is_none_or(|filter| {
                event.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.object_id == event.card
                        && filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                })
            })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardMilled])
    }
    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        if !self.one_or_more {
            return None;
        }
        let event = event.downcast::<CardMilledEvent>()?;
        Some(if self.per_player {
            SimultaneousTriggerKey::PlayerMillingBatch(event.player)
        } else {
            SimultaneousTriggerKey::MillingBatch
        })
    }
    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.matches(event, ctx).then_some(1)
    }
    fn display(&self) -> String {
        let object = self
            .filter
            .as_ref()
            .map(|filter| {
                let mut filter = filter.clone();
                filter.set_plural_object_noun_surface(self.one_or_more);
                filter.description()
            })
            .unwrap_or_else(|| {
                if self.one_or_more {
                    "cards".into()
                } else {
                    "card".into()
                }
            });
        let quantifier = if self.one_or_more {
            "one or more "
        } else {
            "a "
        };
        if self.per_player {
            let player = crate::triggers::describe_player_filter_subject(&self.player);
            let agreement = if self.player == PlayerFilter::You {
                "mill"
            } else {
                "mills"
            };
            format!("Whenever {player} {agreement} {quantifier}{object}")
        } else {
            let copula = if self.one_or_more { "are" } else { "is" };
            format!("Whenever {quantifier}{object} {copula} milled")
        }
    }
}
