//! "Whenever this permanent becomes the target of [spell filter]" trigger.

use crate::events::EventKind;
use crate::events::spells::BecomesTargetedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedBySpellTrigger {
    pub filter: ObjectFilter,
}

impl BecomesTargetedBySpellTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BecomesTargetedBySpellTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        if e.target != ctx.source_id || e.by_ability {
            return false;
        }
        let Some(source) = ctx.game.object(e.source) else {
            return false;
        };
        self.filter.matches(source, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        format!(
            "Whenever this permanent becomes the target of {}",
            self.filter.description()
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
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_aura_spell_on_stack(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.push_to_stack(StackEntry::new(id, controller));
        id
    }

    fn aura_spell_filter() -> ObjectFilter {
        ObjectFilter::default()
            .in_zone(Zone::Stack)
            .with_type(CardType::Enchantment)
            .with_subtype(Subtype::Aura)
    }

    #[test]
    fn matches_when_source_is_matching_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", alice);
        let aura_spell = create_aura_spell_on_stack(&mut game, "Ethereal Armor", bob);

        let trigger = BecomesTargetedBySpellTrigger::new(aura_spell_filter());
        let ctx = TriggerContext::for_source(target, alice, &game);
        let source = game
            .object(aura_spell)
            .expect("aura source spell should exist on stack");
        assert!(
            aura_spell_filter().matches(source, &ctx.filter_ctx, &game),
            "aura spell filter should match source object: {source:#?}"
        );
        let event = TriggerEvent::new(BecomesTargetedEvent::new(target, aura_spell, bob, false));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_targeting_source_is_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", alice);
        let aura_source = create_aura_spell_on_stack(&mut game, "Aura Source", bob);

        let trigger = BecomesTargetedBySpellTrigger::new(aura_spell_filter());
        let ctx = TriggerContext::for_source(target, alice, &game);
        let event = TriggerEvent::new(BecomesTargetedEvent::new(target, aura_source, bob, true));

        assert!(!trigger.matches(&event, &ctx));
    }
}
