//! "When this permanent transforms" trigger.

use crate::events::EventKind;
use crate::events::other::TransformedEvent;
use crate::target::SourceReferenceSurface;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TransformsTrigger {
    pub this_object_surface: Option<SourceReferenceSurface>,
}

impl TransformsTrigger {
    pub fn new() -> Self {
        Self {
            this_object_surface: None,
        }
    }

    pub fn this_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.this_object_surface = Some(surface);
        self
    }

    pub(crate) fn this_subject_text(&self) -> String {
        match &self.this_object_surface {
            Some(SourceReferenceSurface::FullName(text))
            | Some(SourceReferenceSurface::ShortName(text))
            | Some(SourceReferenceSurface::ThisPermanentType(text)) => text.clone(),
            None => "this creature".to_string(),
        }
    }
}

impl Default for TransformsTrigger {
    fn default() -> Self {
        Self::new()
    }
}

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
        format!(
            "Whenever {} transforms into {}",
            self.this_subject_text(),
            self.this_subject_text()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = TransformsTrigger::new();
        assert!(trigger.display().contains("transforms"));
    }
}
