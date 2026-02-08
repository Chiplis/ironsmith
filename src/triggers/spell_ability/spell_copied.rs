//! "Whenever [player] copies [spell]" trigger.

use crate::events::EventKind;
use crate::events::spells::SpellCopiedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCopiedTrigger {
    pub filter: Option<ObjectFilter>,
    pub copier: PlayerFilter,
}

impl SpellCopiedTrigger {
    pub fn new(filter: Option<ObjectFilter>, copier: PlayerFilter) -> Self {
        Self { filter, copier }
    }
}

impl TriggerMatcher for SpellCopiedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCopied {
            return false;
        }
        let Some(e) = event.downcast::<SpellCopiedEvent>() else {
            return false;
        };

        let copier_matches = match &self.copier {
            PlayerFilter::You => e.copier == ctx.controller,
            PlayerFilter::Opponent => e.copier != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.copier == *id,
            _ => true,
        };
        if !copier_matches {
            return false;
        }

        if let Some(ref filter) = self.filter {
            if let Some(obj) = ctx.game.object(e.spell) {
                filter.matches(obj, &ctx.filter_ctx, ctx.game)
            } else {
                false
            }
        } else {
            true
        }
    }

    fn display(&self) -> String {
        let copier_text = match &self.copier {
            PlayerFilter::You => "you copy",
            PlayerFilter::Any => "a player copies",
            PlayerFilter::Opponent => "an opponent copies",
            _ => "someone copies",
        };
        let spell_text = self
            .filter
            .as_ref()
            .map(describe_spell_filter)
            .unwrap_or_else(|| "a spell".to_string());
        format!("Whenever {copier_text} {spell_text}")
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn describe_spell_filter(filter: &ObjectFilter) -> String {
    if filter.card_types.is_empty()
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
    {
        return "a noncreature spell".to_string();
    }

    let fallback = filter.description();
    if fallback == "permanent" {
        "a spell".to_string()
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches_own_copied_spell() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let spell_id = ObjectId::from_raw(2);

        let trigger = SpellCopiedTrigger::new(None, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(SpellCopiedEvent::new(spell_id, alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = SpellCopiedTrigger::new(None, PlayerFilter::You);
        assert!(trigger.display().contains("you copy"));
    }
}
