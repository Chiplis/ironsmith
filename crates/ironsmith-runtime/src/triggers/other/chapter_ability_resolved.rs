//! Saga chapter ability resolution triggers.

use crate::events::EventKind;
use crate::events::other::ChapterAbilityResolvedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::types::Subtype;

#[derive(Debug, Clone, PartialEq)]
pub struct FinalChapterAbilityResolvedTrigger {
    pub filter: ObjectFilter,
}

impl FinalChapterAbilityResolvedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for FinalChapterAbilityResolvedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ChapterAbilityResolved {
            return false;
        }
        let Some(resolved) = event.downcast::<ChapterAbilityResolvedEvent>() else {
            return false;
        };
        if !resolved.final_chapter {
            return false;
        }

        ctx.game
            .object(resolved.saga)
            .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        let subject = if self.filter.subtypes == [Subtype::Saga]
            && self.filter.controller == Some(PlayerFilter::You)
        {
            "a Saga you control".to_string()
        } else {
            self.filter.description()
        };
        format!("Whenever the final chapter ability of {subject} resolves")
    }
}
