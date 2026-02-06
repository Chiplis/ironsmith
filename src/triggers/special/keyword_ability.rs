//! Keyword ability triggers that require specialized event semantics.

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::events::zones::ZoneChangeEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordAbilityTriggerKind {
    Undying,
    Persist,
    Miracle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordAbilityTrigger {
    pub kind: KeywordAbilityTriggerKind,
}

impl KeywordAbilityTrigger {
    pub fn undying() -> Self {
        Self {
            kind: KeywordAbilityTriggerKind::Undying,
        }
    }

    pub fn persist() -> Self {
        Self {
            kind: KeywordAbilityTriggerKind::Persist,
        }
    }

    pub fn miracle() -> Self {
        Self {
            kind: KeywordAbilityTriggerKind::Miracle,
        }
    }
}

impl TriggerMatcher for KeywordAbilityTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        match self.kind {
            KeywordAbilityTriggerKind::Undying => {
                if event.kind() != EventKind::ZoneChange {
                    return false;
                }
                let Some(e) = event.downcast::<ZoneChangeEvent>() else {
                    return false;
                };
                if !e.is_dies() {
                    return false;
                }
                let Some(snapshot) = e.snapshot.as_ref() else {
                    return false;
                };
                snapshot.object_id == ctx.source_id && snapshot.qualifies_for_undying()
            }
            KeywordAbilityTriggerKind::Persist => {
                if event.kind() != EventKind::ZoneChange {
                    return false;
                }
                let Some(e) = event.downcast::<ZoneChangeEvent>() else {
                    return false;
                };
                if !e.is_dies() {
                    return false;
                }
                let Some(snapshot) = e.snapshot.as_ref() else {
                    return false;
                };
                snapshot.object_id == ctx.source_id && snapshot.qualifies_for_persist()
            }
            KeywordAbilityTriggerKind::Miracle => {
                if event.kind() != EventKind::CardsDrawn {
                    return false;
                }
                let Some(e) = event.downcast::<CardsDrawnEvent>() else {
                    return false;
                };
                if e.player != ctx.controller {
                    return false;
                }
                if !e.is_miracle_eligible(ctx.source_id) {
                    return false;
                }

                ctx.game
                    .object(ctx.source_id)
                    .map(|obj| obj.alternative_casts.iter().any(|alt| alt.is_miracle()))
                    .unwrap_or(false)
            }
        }
    }

    fn display(&self) -> String {
        match self.kind {
            KeywordAbilityTriggerKind::Undying => "Undying".to_string(),
            KeywordAbilityTriggerKind::Persist => "Persist".to_string(),
            KeywordAbilityTriggerKind::Miracle => "Miracle".to_string(),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn uses_snapshot(&self) -> bool {
        matches!(
            self.kind,
            KeywordAbilityTriggerKind::Undying | KeywordAbilityTriggerKind::Persist
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        assert_eq!(KeywordAbilityTrigger::undying().display(), "Undying");
        assert_eq!(KeywordAbilityTrigger::persist().display(), "Persist");
        assert_eq!(KeywordAbilityTrigger::miracle().display(), "Miracle");
    }

    #[test]
    fn snapshot_usage() {
        assert!(KeywordAbilityTrigger::undying().uses_snapshot());
        assert!(KeywordAbilityTrigger::persist().uses_snapshot());
        assert!(!KeywordAbilityTrigger::miracle().uses_snapshot());
    }
}
