//! "Whenever a card is put into your graveyard" trigger.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CardPutIntoYourGraveyardTrigger;

impl TriggerMatcher for CardPutIntoYourGraveyardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ZoneChange {
            return false;
        }
        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return false;
        };
        if zc.to != crate::zone::Zone::Graveyard {
            return false;
        }

        // "your graveyard" is ownership-based.
        zc.snapshot
            .as_ref()
            .map(|s| s.owner == ctx.controller)
            .or_else(|| {
                zc.objects
                    .first()
                    .and_then(|&id| ctx.game.object(id))
                    .map(|o| o.owner == ctx.controller)
            })
            .unwrap_or(false)
    }

    fn display(&self) -> String {
        "Whenever a card is put into your graveyard from anywhere".to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = CardPutIntoYourGraveyardTrigger;
        assert!(trigger.display().contains("graveyard"));
    }
}
