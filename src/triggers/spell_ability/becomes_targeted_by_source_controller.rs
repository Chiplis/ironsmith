//! "Whenever [filter] becomes the target of a spell or ability [player] controls" trigger.

use crate::events::EventKind;
use crate::events::spells::BecomesTargetedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedBySourceControllerTrigger {
    pub target_filter: ObjectFilter,
    pub source_controller: PlayerFilter,
}

impl BecomesTargetedBySourceControllerTrigger {
    pub fn new(target_filter: ObjectFilter, source_controller: PlayerFilter) -> Self {
        Self {
            target_filter,
            source_controller,
        }
    }
}

impl TriggerMatcher for BecomesTargetedBySourceControllerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        let Some(target) = ctx.game.object(e.target) else {
            return false;
        };
        if !self
            .target_filter
            .matches(target, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        self.source_controller
            .matches_player(e.source_controller, &ctx.filter_ctx)
    }

    fn display(&self) -> String {
        let controller = match self.source_controller {
            PlayerFilter::You => "you",
            PlayerFilter::Opponent => "an opponent",
            PlayerFilter::Any => "a player",
            _ => "a player",
        };
        format!(
            "Whenever {} becomes the target of a spell or ability {} controls",
            self.target_filter.description(),
            controller
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn matches_when_target_and_controller_match() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Source", alice);
        let spell_source = create_creature(&mut game, "SpellSource", bob);

        let trigger = BecomesTargetedBySourceControllerTrigger::new(
            ObjectFilter::source(),
            PlayerFilter::Opponent,
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(BecomesTargetedEvent::new(source, spell_source, bob, false));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_controller_does_not_match() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Source", alice);
        let spell_source = create_creature(&mut game, "SpellSource", alice);

        let trigger = BecomesTargetedBySourceControllerTrigger::new(
            ObjectFilter::source(),
            PlayerFilter::Opponent,
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(BecomesTargetedEvent::new(
            source,
            spell_source,
            alice,
            false,
        ));

        assert!(!trigger.matches(&event, &ctx));
    }
}
