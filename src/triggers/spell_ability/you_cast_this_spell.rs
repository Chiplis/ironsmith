//! "When you cast this spell" trigger.

use crate::events::EventKind;
use crate::events::spells::SpellCastEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct YouCastThisSpellTrigger;

impl TriggerMatcher for YouCastThisSpellTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCast {
            return false;
        }
        let Some(e) = event.downcast::<SpellCastEvent>() else {
            return false;
        };
        e.spell == ctx.source_id && e.caster == ctx.controller
    }

    fn display(&self) -> String {
        "When you cast this spell".to_string()
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
        let trigger = YouCastThisSpellTrigger;
        assert!(trigger.display().contains("cast this spell"));
    }
}
