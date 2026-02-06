//! Mana Tithe card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Mana Tithe - {W}
/// Instant
/// Counter target spell unless its controller pays {1}.
pub fn mana_tithe() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mana Tithe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays {1}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::executor::execute_effect;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_spell_on_stack(game: &mut GameState, name: &str, caster: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        let obj = Object::from_card(id, &card, caster, Zone::Stack);
        game.add_object(obj);
        game.stack.push(StackEntry {
            object_id: id,
            controller: caster,
            is_ability: false,
            targets: vec![],
            x_value: None,
            ability_effects: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            optional_costs_paid: Default::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: Some(name.to_string()),
            triggering_event: None,
            intervening_if: None,
            chosen_modes: None,
        });
        id
    }

    /// Decision maker that always declines to pay.
    struct DeclineToPayDecisionMaker;

    impl DecisionMaker for DeclineToPayDecisionMaker {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            false
        }
    }

    /// Decision maker that always accepts to pay.
    struct AcceptToPayDecisionMaker;

    impl DecisionMaker for AcceptToPayDecisionMaker {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_mana_tithe_basic_properties() {
        let def = mana_tithe();
        assert_eq!(def.name(), "Mana Tithe");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_mana_tithe_is_instant() {
        let def = mana_tithe();
        assert!(def.card.is_instant());
    }

    #[test]
    fn test_mana_tithe_is_white() {
        let def = mana_tithe();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_mana_tithe_has_one_effect() {
        let def = mana_tithe();
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_mana_tithe_counters_spell_when_opponent_cant_pay() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell with no mana in pool
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Bob can't pay {1}, spell is countered
        assert_eq!(result.result, crate::effect::EffectResult::Resolved);
        assert!(game.stack.is_empty());
        // Spell should be in Bob's graveyard
        let bob_gy = &game.player(bob).unwrap().graveyard;
        assert_eq!(bob_gy.len(), 1);
    }

    #[test]
    fn test_mana_tithe_counters_spell_when_opponent_declines_to_pay() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell and has mana to pay
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);
        // Give Bob mana to pay (Colorless mana)
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let source = game.new_object_id();
        let mut dm = DeclineToPayDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)])
            .with_decision_maker(&mut dm);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Bob declines to pay, spell is countered
        assert_eq!(result.result, crate::effect::EffectResult::Resolved);
        assert!(game.stack.is_empty());
    }

    #[test]
    fn test_mana_tithe_does_not_counter_when_opponent_pays() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell and has mana to pay
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);
        // Give Bob mana to pay {1}
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let source = game.new_object_id();
        let mut dm = AcceptToPayDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)])
            .with_decision_maker(&mut dm);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Bob pays {1}, spell is NOT countered
        assert_eq!(result.result, crate::effect::EffectResult::Declined);
        assert_eq!(game.stack.len(), 1);
        // Bob's mana pool should be empty now
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_mana_tithe_does_not_counter_uncounterable_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_id = create_spell_on_stack(&mut game, "Carnage Tyrant", bob);

        // Add "can't be countered" to the spell
        if let Some(obj) = game.object_mut(spell_id) {
            obj.abilities.push(Ability {
                kind: AbilityKind::Static(StaticAbility::uncounterable()),
                functional_zones: vec![Zone::Stack],
                text: Some("can't be countered".to_string()),
            });
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Spell can't be countered
        assert_eq!(result.result, crate::effect::EffectResult::Protected);
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_mana_tithe_requires_exactly_one_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob has no mana, then gets exactly 1 colorless
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let source = game.new_object_id();
        let mut dm = AcceptToPayDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)])
            .with_decision_maker(&mut dm);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Bob can pay exactly {1}
        assert_eq!(result.result, crate::effect::EffectResult::Declined);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_mana_tithe_can_be_paid_with_any_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);
        // Give Bob green mana (should still pay for {1})
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let source = game.new_object_id();
        let mut dm = AcceptToPayDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)])
            .with_decision_maker(&mut dm);

        let def = mana_tithe();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Mana Tithe should have spell effects")
            .first()
            .expect("Mana Tithe should have a spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // Bob can pay {1} with green mana
        assert_eq!(result.result, crate::effect::EffectResult::Declined);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_mana_tithe_oracle_text() {
        let def = mana_tithe();
        assert!(def.card.oracle_text.contains("Counter target spell"));
        assert!(def.card.oracle_text.contains("unless"));
        assert!(def.card.oracle_text.contains("pays"));
        assert!(def.card.oracle_text.contains("{1}"));
    }
}
