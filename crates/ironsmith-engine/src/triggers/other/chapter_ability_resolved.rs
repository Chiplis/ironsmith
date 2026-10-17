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

        if let Some(snapshot) = event.source_snapshot() {
            return self
                .filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
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

/// "Whenever the final chapter ability of a Saga you control triggers"
/// (CR 714.2d): the triggering ability is a chapter ability of a matching
/// source whose chapter numbers include that source's greatest chapter number.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalChapterAbilityTriggersTrigger {
    pub filter: ObjectFilter,
}

impl FinalChapterAbilityTriggersTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for FinalChapterAbilityTriggersTrigger {
    fn canonical_model(&self) -> Option<ironsmith_core::trigger_model::Trigger> {
        Some(ironsmith_core::trigger_model::Trigger::final_chapter_ability_triggers(
            self.filter.clone(),
        ))
    }

    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityTriggered {
            return false;
        }
        let Some(triggered) = event.downcast::<crate::events::spells::AbilityTriggeredEvent>()
        else {
            return false;
        };
        let live = ctx.game.object(triggered.source);
        let source_matches = live
            .is_some_and(|source| self.filter.matches(source, &ctx.filter_ctx, ctx.game))
            || triggered.source_snapshot.as_ref().is_some_and(|snapshot| {
                self.filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
            });
        if !source_matches {
            return false;
        }
        let abilities = ctx
            .game
            .current_abilities(triggered.source)
            .filter(|_| live.is_some())
            .or_else(|| {
                triggered
                    .source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.abilities.as_ref().clone())
            })
            .unwrap_or_default();
        let Some(final_chapter) = crate::game_loop::final_chapter_number_from_abilities(&abilities)
        else {
            return false;
        };
        abilities.iter().any(|ability| {
            let crate::ability::AbilityKind::Triggered(ability) = &ability.kind else {
                return false;
            };
            crate::triggers::compute_trigger_identity(ability) == triggered.trigger_identity
                && ability
                    .trigger
                    .saga_chapters()
                    .is_some_and(|chapters| chapters.contains(&final_chapter))
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::AbilityTriggered])
    }

    fn display(&self) -> String {
        let subject = if self.filter.subtypes == [Subtype::Saga]
            && self.filter.controller == Some(PlayerFilter::You)
        {
            "a Saga you control".to_string()
        } else {
            self.filter.description()
        };
        format!("Whenever the final chapter ability of {subject} triggers")
    }
}
