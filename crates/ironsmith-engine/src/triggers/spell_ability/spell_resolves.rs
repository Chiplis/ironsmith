//! Controller-aware spell resolution matcher; no cast or zone-change proxy.
use crate::events::{EventKind, spells::SpellResolvedEvent};
use crate::filter::PlayerFilterExt as _;
use crate::target::PlayerFilter;
use crate::triggers::{
    TriggerEvent,
    matcher_trait::{TriggerContext, TriggerMatcher},
};
#[derive(Debug, Clone, PartialEq)]
pub struct SpellResolvesTrigger {
    pub controller: PlayerFilter,
}
impl SpellResolvesTrigger {
    pub fn new(controller: PlayerFilter) -> Self {
        Self { controller }
    }
}
impl TriggerMatcher for SpellResolvesTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellResolved {
            return false;
        }
        event
            .downcast::<SpellResolvedEvent>()
            .is_some_and(|resolved| {
                self.controller
                    .matches_player(resolved.controller, &ctx.filter_ctx)
            })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::SpellResolved])
    }
    fn display(&self) -> String {
        let subject = match self.controller {
            PlayerFilter::You => "a spell you control".into(),
            PlayerFilter::Opponent => "a spell an opponent controls".into(),
            PlayerFilter::Any => "a spell".into(),
            _ => format!("a spell controlled by {}", self.controller.description()),
        };
        format!("Whenever {subject} resolves")
    }
}
