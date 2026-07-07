//! "Whenever this creature attacks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::types::CardType;
use crate::zone::Zone;

/// Trigger that fires when the source creature attacks.
///
/// Used by cards like Goblin Guide, Geist of Saint Traft, and Hero of Bladehold.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksTrigger;

impl TriggerMatcher for ThisAttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        e.attacker == ctx.source_id
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        "Whenever this creature attacks".to_string()
    }
}

/// Trigger that fires when the source creature attacks a player who controls at
/// least a required number of matching permanents.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksPlayerWhoControlsAtLeastTrigger {
    pub count: usize,
    pub filter: ObjectFilter,
}

impl ThisAttacksPlayerWhoControlsAtLeastTrigger {
    pub fn new(count: usize, filter: ObjectFilter) -> Self {
        Self { count, filter }
    }
}

impl TriggerMatcher for ThisAttacksPlayerWhoControlsAtLeastTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if e.attacker != ctx.source_id {
            return false;
        }
        let crate::events::combat::AttackEventTarget::Player(defending_player) = e.target else {
            return false;
        };
        let controlled_count = ctx
            .game
            .battlefield
            .iter()
            .filter_map(|object_id| ctx.game.object(*object_id))
            .filter(|object| ctx.game.controller_of(object) == defending_player)
            .filter(|object| {
                object.zone == Zone::Battlefield
                    && self.filter.matches(object, &ctx.filter_ctx, ctx.game)
            })
            .count();
        controlled_count >= self.count
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        format!(
            "Whenever this creature attacks a player who controls {} or more {}",
            count_word(self.count),
            controlled_filter_noun(&self.filter),
        )
    }
}

fn count_word(count: usize) -> String {
    match count {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        9 => "nine".to_string(),
        10 => "ten".to_string(),
        _ => count.to_string(),
    }
}

fn controlled_filter_noun(filter: &ObjectFilter) -> String {
    if filter.card_types == [CardType::Land]
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.any_of.is_empty()
    {
        return "lands".to_string();
    }
    filter.description()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_matches_own_attack() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisAttacksTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_other_attack() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);

        let trigger = ThisAttacksTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(other_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = ThisAttacksTrigger;
        assert!(trigger.display().contains("attacks"));
    }
}
