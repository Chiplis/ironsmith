//! "Whenever [filter] attacks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks.
///
/// Used by cards that care about other creatures attacking.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksTrigger {
    /// Filter for creatures that trigger this ability.
    pub filter: ObjectFilter,
}

impl AttacksTrigger {
    /// Create a new attacks trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create an attacks trigger for any creature.
    pub fn any_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl TriggerMatcher for AttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.attacker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever {} attacks", self.filter.description())
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_creature_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        let trigger = AttacksTrigger::any_creature();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            creature_id,
            AttackEventTarget::Player(bob),
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = AttacksTrigger::any_creature();
        assert!(trigger.display().contains("attacks"));
    }
}
