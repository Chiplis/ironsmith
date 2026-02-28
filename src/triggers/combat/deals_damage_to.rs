//! "Whenever [source filter] deals damage to [target filter]" trigger.

use crate::events::DamageEvent;
use crate::events::EventKind;
use crate::game_event::DamageTarget;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsDamageToTrigger {
    pub source_filter: ObjectFilter,
    pub target_filter: ObjectFilter,
    pub combat_only: bool,
}

impl DealsDamageToTrigger {
    pub fn new(source_filter: ObjectFilter, target_filter: ObjectFilter) -> Self {
        Self {
            source_filter,
            target_filter,
            combat_only: false,
        }
    }

    pub fn combat_only(source_filter: ObjectFilter, target_filter: ObjectFilter) -> Self {
        Self {
            source_filter,
            target_filter,
            combat_only: true,
        }
    }
}

impl TriggerMatcher for DealsDamageToTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if self.combat_only && !damage.is_combat {
            return false;
        }
        let Some(source_obj) = ctx.game.object(damage.source) else {
            return false;
        };
        if !self
            .source_filter
            .matches(source_obj, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        let Some(target_obj) = ctx.game.object(target_id) else {
            return false;
        };
        self.target_filter
            .matches(target_obj, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        if self.combat_only {
            format!(
                "Whenever {} deals combat damage to {}",
                self.source_filter.description(),
                self.target_filter.description()
            )
        } else {
            format!(
                "Whenever {} deals damage to {}",
                self.source_filter.description(),
                self.target_filter.description()
            )
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_combat_damage_to_matching_target() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", bob);

        let trigger =
            DealsDamageToTrigger::combat_only(ObjectFilter::creature(), ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(DamageEvent::new(
            source,
            DamageTarget::Object(target),
            2,
            true,
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_noncombat_damage_when_combat_only() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", bob);

        let trigger =
            DealsDamageToTrigger::combat_only(ObjectFilter::creature(), ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(DamageEvent::new(
            source,
            DamageTarget::Object(target),
            2,
            false,
        ));

        assert!(!trigger.matches(&event, &ctx));
    }
}
