//! Rebuff the Wicked card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

#[cfg(test)]
use crate::ability::AbilityKind;
#[cfg(test)]
use crate::effect::{Effect, EffectResult};
#[cfg(test)]
use crate::executor::{ExecutionContext, ResolvedTarget};
#[cfg(test)]
use crate::game_state::{GameState, Target};
#[cfg(test)]
use crate::target::{ChooseSpec, ObjectFilter};
#[cfg(test)]
use crate::zone::Zone;

/// Rebuff the Wicked - {W}
/// Instant
/// Counter target spell that targets a permanent you control.
pub fn rebuff_the_wicked() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Rebuff the Wicked")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell that targets a permanent you control.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::Color;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_spell_on_stack_targeting(
        game: &mut GameState,
        name: &str,
        caster: PlayerId,
        targets: Vec<Target>,
    ) -> ObjectId {
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
            targets,
            x_value: None,
            ability_effects: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            optional_costs_paid: Default::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_snapshot: None,
            source_name: Some(name.to_string()),
            triggering_event: None,
            intervening_if: None,
            keyword_payment_contributions: vec![],
            crew_contributors: vec![],
            chosen_modes: None,
        });
        id
    }

    fn rebuff_effect() -> Effect {
        Effect::counter(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::spell().targeting_object(ObjectFilter::permanent().you_control()),
        )))
    }

    fn rebuff_filter() -> ObjectFilter {
        ObjectFilter::spell().targeting_object(ObjectFilter::permanent().you_control())
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_rebuff_the_wicked_basic_properties() {
        let def = rebuff_the_wicked();
        assert_eq!(def.name(), "Rebuff the Wicked");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_rebuff_the_wicked_is_instant() {
        let def = rebuff_the_wicked();
        assert!(def.card.is_instant());
    }

    #[test]
    fn test_rebuff_the_wicked_is_white() {
        let def = rebuff_the_wicked();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_rebuff_the_wicked_has_one_effect() {
        let def = rebuff_the_wicked();
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_rebuff_counters_spell_targeting_your_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice controls a creature
        let alice_creature = create_creature(&mut game, "Soldier", alice);

        // Bob casts a spell targeting Alice's creature
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Lightning Bolt",
            bob,
            vec![Target::Object(alice_creature)],
        );

        // Alice casts Rebuff the Wicked targeting the spell
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = rebuff_effect();
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Spell should be countered
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.stack.is_empty());
        // Spell should be in Bob's graveyard
        let bob_gy = &game.player(bob).unwrap().graveyard;
        assert_eq!(bob_gy.len(), 1);
    }

    #[test]
    fn test_rebuff_fails_on_spell_not_targeting_your_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob controls a creature
        let bob_creature = create_creature(&mut game, "Goblin", bob);

        // Bob casts a spell targeting his OWN creature (pump spell or something)
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Giant Growth",
            bob,
            vec![Target::Object(bob_creature)],
        );

        // Filter should reject spells not targeting your permanents
        let filter_ctx = game.filter_context_for(alice, None);
        let spell_obj = game.object(spell_id).unwrap();
        assert!(
            !rebuff_filter().matches(spell_obj, &filter_ctx, &game),
            "Spell targeting opponent's permanent should not match filter"
        );
    }

    #[test]
    fn test_rebuff_fails_on_spell_targeting_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell targeting Alice (the player)
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Lightning Bolt",
            bob,
            vec![Target::Player(alice)],
        );

        // Filter should reject spells targeting a player
        let filter_ctx = game.filter_context_for(alice, None);
        let spell_obj = game.object(spell_id).unwrap();
        assert!(
            !rebuff_filter().matches(spell_obj, &filter_ctx, &game),
            "Spell targeting a player should not match filter"
        );
    }

    #[test]
    fn test_rebuff_counters_spell_with_multiple_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice controls a creature
        let alice_creature = create_creature(&mut game, "Soldier", alice);
        // Bob controls a creature
        let bob_creature = create_creature(&mut game, "Goblin", bob);

        // Bob casts a spell targeting both creatures
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Arc Lightning",
            bob,
            vec![Target::Object(alice_creature), Target::Object(bob_creature)],
        );

        // Alice casts Rebuff the Wicked (should work since one target is hers)
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = rebuff_effect();
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Spell should be countered (at least one target is Alice's permanent)
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.stack.is_empty());
    }

    #[test]
    fn test_rebuff_does_not_counter_uncounterable_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice controls a creature
        let alice_creature = create_creature(&mut game, "Soldier", alice);

        // Bob casts an uncounterable spell targeting Alice's creature
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Abrupt Decay",
            bob,
            vec![Target::Object(alice_creature)],
        );

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

        let effect = rebuff_effect();
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Spell can't be countered
        assert_eq!(result.result, EffectResult::Protected);
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_rebuff_fails_on_spell_with_no_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell with no targets
        let spell_id = create_spell_on_stack_targeting(&mut game, "Wrath of God", bob, vec![]);

        // Filter should reject spells with no targets
        let filter_ctx = game.filter_context_for(alice, None);
        let spell_obj = game.object(spell_id).unwrap();
        assert!(
            !rebuff_filter().matches(spell_obj, &filter_ctx, &game),
            "Spell with no targets should not match filter"
        );
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_rebuff_the_wicked_oracle_text() {
        let def = rebuff_the_wicked();
        assert!(def.card.oracle_text.contains("Counter target spell"));
        assert!(
            def.card
                .oracle_text
                .contains("targets a permanent you control")
        );
    }
}
