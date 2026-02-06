//! Blacksmith's Skill card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::{Condition, Effect, EffectId, EffectPredicate, Until};
use crate::effects::GrantAbilitiesTargetEffect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;

/// Blacksmith's Skill - {W}
/// Instant
/// Target permanent gains hexproof and indestructible until end of turn.
/// If it's an artifact creature, it gets +2/+2 until end of turn.
pub fn blacksmiths_skill() -> CardDefinition {
    let grant = Effect::with_id(
        0,
        Effect::new(GrantAbilitiesTargetEffect::new(
            ChooseSpec::permanent(),
            vec![StaticAbility::hexproof(), StaticAbility::indestructible()],
            Until::EndOfTurn,
        ))
        .tag("target"),
    );
    let pump = Effect::if_then(
        EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::conditional(
            Condition::TaggedObjectMatches(
                "target".into(),
                ObjectFilter::artifact().with_all_type(CardType::Creature),
            ),
            vec![Effect::pump(
                2,
                2,
                ChooseSpec::Tagged("target".into()),
                Until::EndOfTurn,
            )],
            vec![],
        )],
    );

    CardDefinitionBuilder::new(CardId::new(), "Blacksmith's Skill")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![grant, pump])
        .oracle_text(
            "Target permanent gains hexproof and indestructible until end of turn. \
             If it's an artifact creature, it gets +2/+2 until end of turn.",
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Creature])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_artifact_creature(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_artifact(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_blacksmiths_skill_basic_properties() {
        let def = blacksmiths_skill();
        assert_eq!(def.name(), "Blacksmith's Skill");
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 1); // {W} = 1
    }

    #[test]
    fn test_has_spell_effect() {
        let def = blacksmiths_skill();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_grants_hexproof_and_indestructible_to_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a regular creature
        let creature_id = create_creature(&mut game, "Test Creature", alice);

        // Verify it doesn't have hexproof or indestructible initially
        let creature = game.object(creature_id).unwrap();
        assert!(!creature.has_static_ability(&StaticAbility::hexproof()));
        assert!(!creature.has_static_ability(&StaticAbility::indestructible()));

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        // Now check for the granted abilities through the continuous effects
        game.update_static_ability_effects();

        // The abilities should be granted via continuous effects
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::hexproof()),
            "Creature should have hexproof"
        );
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::indestructible()),
            "Creature should have indestructible"
        );
    }

    #[test]
    fn test_grants_abilities_to_artifact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a non-creature artifact
        let artifact_id = create_artifact(&mut game, "Test Artifact", alice);

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(artifact_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        game.update_static_ability_effects();

        // Should have hexproof and indestructible
        assert!(
            game.object_has_ability(artifact_id, &StaticAbility::hexproof()),
            "Artifact should have hexproof"
        );
        assert!(
            game.object_has_ability(artifact_id, &StaticAbility::indestructible()),
            "Artifact should have indestructible"
        );
    }

    #[test]
    fn test_artifact_creature_gets_pump() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create an artifact creature
        let creature_id = create_artifact_creature(&mut game, "Steel Golem", alice);

        // Set base P/T for testing
        {
            let creature = game.object_mut(creature_id).unwrap();
            creature.base_power = Some(crate::card::PtValue::Fixed(2));
            creature.base_toughness = Some(crate::card::PtValue::Fixed(2));
        }

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        game.update_static_ability_effects();

        // Should have hexproof and indestructible
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::hexproof()),
            "Artifact creature should have hexproof"
        );
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::indestructible()),
            "Artifact creature should have indestructible"
        );

        // Should have +2/+2 (calculated power/toughness should be 4/4)
        assert_eq!(
            game.calculated_power(creature_id),
            Some(4),
            "Artifact creature should have +2 power (2 + 2 = 4)"
        );
        assert_eq!(
            game.calculated_toughness(creature_id),
            Some(4),
            "Artifact creature should have +2 toughness (2 + 2 = 4)"
        );
    }

    #[test]
    fn test_regular_creature_does_not_get_pump() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a regular (non-artifact) creature
        let creature_id = create_creature(&mut game, "Soldier", alice);

        // Set base P/T for testing
        {
            let creature = game.object_mut(creature_id).unwrap();
            creature.base_power = Some(crate::card::PtValue::Fixed(2));
            creature.base_toughness = Some(crate::card::PtValue::Fixed(2));
        }

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        game.update_static_ability_effects();

        // Should have hexproof and indestructible
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::hexproof()),
            "Creature should have hexproof"
        );
        assert!(
            game.object_has_ability(creature_id, &StaticAbility::indestructible()),
            "Creature should have indestructible"
        );

        // Should NOT have the +2/+2 pump (still 2/2)
        assert_eq!(
            game.calculated_power(creature_id),
            Some(2),
            "Regular creature should NOT get +2 power (still 2)"
        );
        assert_eq!(
            game.calculated_toughness(creature_id),
            Some(2),
            "Regular creature should NOT get +2 toughness (still 2)"
        );
    }

    #[test]
    fn test_non_creature_artifact_does_not_get_pump() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a non-creature artifact
        let artifact_id = create_artifact(&mut game, "Sol Ring", alice);

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(artifact_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        game.update_static_ability_effects();

        // Should have hexproof and indestructible
        assert!(
            game.object_has_ability(artifact_id, &StaticAbility::hexproof()),
            "Artifact should have hexproof"
        );
        assert!(
            game.object_has_ability(artifact_id, &StaticAbility::indestructible()),
            "Artifact should have indestructible"
        );

        // Verify it's not a creature (so no P/T)
        assert!(game.calculated_power(artifact_id).is_none());
        assert!(game.calculated_toughness(artifact_id).is_none());
    }

    #[test]
    fn test_creates_continuous_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a regular creature
        let creature_id = create_creature(&mut game, "Test Creature", alice);

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        // Should have created 2 continuous effects (hexproof + indestructible)
        let cont_effects = game.continuous_effects.effects_sorted();
        assert_eq!(cont_effects.len(), 2, "Should have 2 continuous effects");
    }

    #[test]
    fn test_artifact_creature_creates_three_continuous_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create an artifact creature
        let creature_id = create_artifact_creature(&mut game, "Steel Golem", alice);

        // Execute the spell effect
        let def = blacksmiths_skill();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let effects = def.spell_effect.as_ref().unwrap();
        for effect in effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        // Should have created 3 continuous effects (hexproof + indestructible + pump)
        let cont_effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            cont_effects.len(),
            3,
            "Should have 3 continuous effects for artifact creature"
        );
    }
}
