//! "When this permanent transforms" trigger.

use crate::events::EventKind;
use crate::events::other::TransformedEvent;
use crate::target::SourceReferenceSurface;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TransformsTrigger {
    pub this_object_surface: Option<SourceReferenceSurface>,
    pub destination_name: Option<String>,
}

impl TransformsTrigger {
    pub fn new() -> Self {
        Self {
            this_object_surface: None,
            destination_name: None,
        }
    }

    pub fn this_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.this_object_surface = Some(surface);
        self
    }

    pub fn destination_name(mut self, destination_name: Option<String>) -> Self {
        self.destination_name = destination_name;
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

    pub(crate) fn destination_text(&self) -> String {
        self.destination_name
            .clone()
            .unwrap_or_else(|| self.this_subject_text())
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
        if e.permanent != ctx.source_id {
            return false;
        }
        if let Some(destination_name) = self.destination_name.as_ref() {
            return ctx
                .game
                .object(e.permanent)
                .is_some_and(|object| object.name == *destination_name);
        }
        true
    }

    fn display(&self) -> String {
        if self.destination_name.is_some() {
            return format!(
                "Whenever {} transforms into {}",
                self.this_subject_text(),
                self.destination_text()
            );
        }
        format!("When {} transforms", self.this_subject_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    #[test]
    fn test_display() {
        let trigger = TransformsTrigger::new();
        assert!(trigger.display().contains("transforms"));
    }

    #[test]
    fn destination_name_filters_transformed_face() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(98_701), "Back Face").build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(
            TransformedEvent::new(source),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);

        assert!(
            TransformsTrigger::new()
                .destination_name(Some("Back Face".to_string()))
                .matches(&event, &ctx)
        );
        assert!(
            !TransformsTrigger::new()
                .destination_name(Some("Front Face".to_string()))
                .matches(&event, &ctx)
        );
    }
}
