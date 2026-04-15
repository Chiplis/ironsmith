//! "When this permanent transforms" trigger.

use crate::events::EventKind;
use crate::events::other::TransformedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TransformsTrigger;

impl TriggerMatcher for TransformsTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Transformed {
            return false;
        }
        let Some(e) = event.downcast::<TransformedEvent>() else {
            return false;
        };
        e.permanent == ctx.source_id
    }

    fn display(&self) -> String {
        "When this permanent transforms".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = TransformsTrigger;
        assert!(trigger.display().contains("transforms"));
    }
}
