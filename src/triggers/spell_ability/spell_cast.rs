//! "Whenever [player] casts [spell]" trigger.

use crate::events::EventKind;
use crate::events::spells::SpellCastEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCastTrigger {
    pub filter: Option<ObjectFilter>,
    pub caster: PlayerFilter,
}

impl SpellCastTrigger {
    pub fn new(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self { filter, caster }
    }

    pub fn you_cast_any() -> Self {
        Self::new(None, PlayerFilter::You)
    }

    pub fn any_cast_any() -> Self {
        Self::new(None, PlayerFilter::Any)
    }
}

impl TriggerMatcher for SpellCastTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCast {
            return false;
        }
        let Some(e) = event.downcast::<SpellCastEvent>() else {
            return false;
        };

        // Check caster filter
        let caster_matches = match &self.caster {
            PlayerFilter::You => e.caster == ctx.controller,
            PlayerFilter::Opponent => e.caster != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.caster == *id,
            _ => true,
        };

        if !caster_matches {
            return false;
        }

        // Check spell filter if present
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
        let caster_text = match &self.caster {
            PlayerFilter::You => "you cast",
            PlayerFilter::Any => "a player casts",
            PlayerFilter::Opponent => "an opponent casts",
            _ => "someone casts",
        };
        let spell_text = self
            .filter
            .as_ref()
            .map(|f| f.description())
            .unwrap_or_else(|| "a spell".to_string());
        format!("Whenever {} {}", caster_text, spell_text)
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches_own_spell() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let spell_id = ObjectId::from_raw(2);

        let trigger = SpellCastTrigger::you_cast_any();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(SpellCastEvent::new(spell_id, alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = SpellCastTrigger::you_cast_any();
        assert!(trigger.display().contains("you cast"));
    }
}
