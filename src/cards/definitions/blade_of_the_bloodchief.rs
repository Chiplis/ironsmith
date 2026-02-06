//! Blade of the Bloodchief card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Blade of the Bloodchief - {1}
/// Artifact — Equipment
/// Whenever a creature dies, put a +1/+1 counter on equipped creature.
/// If equipped creature is a Vampire, put two +1/+1 counters on it instead.
/// Equip {1}
pub fn blade_of_the_bloodchief() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Blade of the Bloodchief")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Whenever a creature dies, put a +1/+1 counter on equipped creature. \
             If equipped creature is a Vampire, put two +1/+1 counters on it instead.\n\
             Equip {1}",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ability::ActivationTiming;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::{CounterType, Object};
    use crate::target::ChooseSpec;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        subtypes: Vec<Subtype>,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_equipment(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = blade_of_the_bloodchief();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn attach_equipment(game: &mut GameState, equipment_id: ObjectId, creature_id: ObjectId) {
        if let Some(equipment) = game.object_mut(equipment_id) {
            equipment.attached_to = Some(creature_id);
        }
        if let Some(creature) = game.object_mut(creature_id) {
            creature.attachments.push(equipment_id);
        }
    }

    fn execute_blade_trigger(game: &mut GameState, controller: PlayerId, source: ObjectId) {
        let def = blade_of_the_bloodchief();
        let triggered = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Blade should have a triggered ability");
        let AbilityKind::Triggered(triggered) = &triggered.kind else {
            unreachable!("Expected triggered ability");
        };

        let mut ctx = ExecutionContext::new_default(source, controller);
        for effect in &triggered.effects {
            effect.0.execute(game, &mut ctx).unwrap();
        }
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_blade_of_the_bloodchief_basic_properties() {
        let def = blade_of_the_bloodchief();
        assert_eq!(def.name(), "Blade of the Bloodchief");
        assert!(def.card.is_artifact());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_has_equipment_subtype() {
        let def = blade_of_the_bloodchief();
        assert!(def.card.subtypes.contains(&Subtype::Equipment));
    }

    #[test]
    fn test_has_correct_number_of_abilities() {
        let def = blade_of_the_bloodchief();
        // Should have 2 abilities: triggered ability + equip
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Triggered Ability Structure Tests
    // ========================================

    #[test]
    fn test_has_creature_dies_trigger() {
        let def = blade_of_the_bloodchief();

        let trigger_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(trigger_ability.is_some(), "Should have a triggered ability");

        if let AbilityKind::Triggered(triggered) = &trigger_ability.unwrap().kind {
            // Verify trigger condition is creature dies (now using Trigger struct)
            assert!(
                triggered.trigger.display().contains("dies"),
                "Should trigger when a creature dies"
            );
            // Tag + conditional counter placement
            assert_eq!(triggered.effects.len(), 2);
        } else {
            panic!("Expected triggered ability");
        }
    }

    // ========================================
    // Equip Ability Structure Tests
    // ========================================

    #[test]
    fn test_has_equip_ability() {
        let def = blade_of_the_bloodchief();

        let equip_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(
            equip_ability.is_some(),
            "Should have an activated ability (equip)"
        );

        if let AbilityKind::Activated(activated) = &equip_ability.unwrap().kind {
            // Verify equip cost is {1}
            let mana_cost = activated.mana_cost.mana_cost();
            assert!(mana_cost.is_some(), "Equip should have a mana cost");
            assert_eq!(mana_cost.unwrap().mana_value(), 1, "Equip cost should be 1");

            // Verify timing is sorcery speed
            assert_eq!(
                activated.timing,
                ActivationTiming::SorcerySpeed,
                "Equip is sorcery speed"
            );

            // Verify targets creature you control
            assert_eq!(activated.choices.len(), 1);
            let target_spec = match &activated.choices[0] {
                ChooseSpec::Target(inner) => inner.as_ref(),
                other => other,
            };
            if let ChooseSpec::Object(filter) = target_spec {
                assert!(filter.card_types.contains(&CardType::Creature));
                assert!(matches!(
                    filter.controller,
                    Some(crate::target::PlayerFilter::You)
                ));
            } else {
                panic!("Equip should target an object");
            }
        } else {
            panic!("Expected activated ability");
        }
    }

    // ========================================
    // Trigger Effect Tests - Non-Vampire Creature
    // ========================================

    #[test]
    fn test_trigger_puts_one_counter_on_non_vampire() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a non-vampire creature and attach the blade
        let creature_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Verify initial state
        {
            let creature = game.object(creature_id).unwrap();
            assert_eq!(creature.counters.get(&CounterType::PlusOnePlusOne), None);
        }

        // Execute the trigger effect
        execute_blade_trigger(&mut game, alice, equipment_id);

        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&1),
            "Non-vampire should get 1 +1/+1 counter"
        );
    }

    // ========================================
    // Trigger Effect Tests - Vampire Creature
    // ========================================

    #[test]
    fn test_trigger_puts_two_counters_on_vampire() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a vampire creature and attach the blade
        let vampire_id = create_creature(&mut game, "Vampire", vec![Subtype::Vampire], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, vampire_id);

        // Execute the trigger effect
        execute_blade_trigger(&mut game, alice, equipment_id);

        let vampire = game.object(vampire_id).unwrap();
        assert_eq!(
            vampire.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2),
            "Vampire should get 2 +1/+1 counters"
        );
    }

    // ========================================
    // Trigger Effect Tests - Edge Cases
    // ========================================

    #[test]
    fn test_trigger_does_nothing_when_not_equipped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create equipment but don't attach it
        let equipment_id = create_equipment(&mut game, alice);

        // Execute the trigger effect
        execute_blade_trigger(&mut game, alice, equipment_id);

        assert!(
            game.object(equipment_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .is_none(),
            "Equipment should not receive counters"
        );
    }

    #[test]
    fn test_trigger_does_nothing_when_equipped_creature_gone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create equipment with attached_to pointing to non-existent creature
        let equipment_id = create_equipment(&mut game, alice);
        let fake_creature_id = ObjectId::from_raw(99999);

        if let Some(equipment) = game.object_mut(equipment_id) {
            equipment.attached_to = Some(fake_creature_id);
        }

        // Execute the trigger effect
        execute_blade_trigger(&mut game, alice, equipment_id);

        assert!(
            game.object(equipment_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .is_none(),
            "Equipment should not receive counters"
        );
    }

    #[test]
    fn test_trigger_accumulates_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and attach the blade
        let creature_id = create_creature(&mut game, "Warrior", vec![Subtype::Warrior], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Trigger multiple times (simulating multiple creature deaths)
        for _ in 0..3 {
            execute_blade_trigger(&mut game, alice, equipment_id);
        }

        // Should have accumulated 3 counters
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&3),
            "Should accumulate 3 counters after 3 triggers"
        );
    }

    #[test]
    fn test_trigger_accumulates_counters_on_vampire() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a vampire and attach the blade
        let vampire_id = create_creature(&mut game, "Vampire Lord", vec![Subtype::Vampire], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, vampire_id);

        // Trigger multiple times
        for _ in 0..3 {
            execute_blade_trigger(&mut game, alice, equipment_id);
        }

        // Should have accumulated 6 counters (2 per trigger × 3 triggers)
        let vampire = game.object(vampire_id).unwrap();
        assert_eq!(
            vampire.counters.get(&CounterType::PlusOnePlusOne),
            Some(&6),
            "Vampire should accumulate 6 counters after 3 triggers"
        );
    }

    // ========================================
    // Power/Toughness Tests with Counters
    // ========================================

    #[test]
    fn test_counters_affect_power_toughness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a 2/2 creature and attach the blade
        let creature_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Verify initial P/T
        {
            let creature = game.object(creature_id).unwrap();
            assert_eq!(creature.power(), Some(2));
            assert_eq!(creature.toughness(), Some(2));
        }

        // Trigger the effect twice
        for _ in 0..2 {
            execute_blade_trigger(&mut game, alice, equipment_id);
        }

        // Should now be 4/4 (2 base + 2 counters)
        let creature = game.object(creature_id).unwrap();
        assert_eq!(creature.power(), Some(4));
        assert_eq!(creature.toughness(), Some(4));
    }

    #[test]
    fn test_vampire_gets_more_pt_from_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a 2/2 vampire and attach the blade
        let vampire_id = create_creature(&mut game, "Vampire", vec![Subtype::Vampire], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, vampire_id);

        // Verify initial P/T
        {
            let vampire = game.object(vampire_id).unwrap();
            assert_eq!(vampire.power(), Some(2));
            assert_eq!(vampire.toughness(), Some(2));
        }

        // Trigger the effect twice
        for _ in 0..2 {
            execute_blade_trigger(&mut game, alice, equipment_id);
        }

        // Should now be 6/6 (2 base + 4 counters from 2 triggers × 2 each)
        let vampire = game.object(vampire_id).unwrap();
        assert_eq!(vampire.power(), Some(6));
        assert_eq!(vampire.toughness(), Some(6));
    }

    // ========================================
    // Equipment Movement Tests
    // ========================================

    #[test]
    fn test_trigger_works_after_moving_equipment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two creatures and the equipment
        let creature1_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let creature2_id = create_creature(&mut game, "Knight", vec![Subtype::Knight], alice);
        let equipment_id = create_equipment(&mut game, alice);

        // Attach to first creature
        attach_equipment(&mut game, equipment_id, creature1_id);

        // Trigger - should put counter on creature1
        execute_blade_trigger(&mut game, alice, equipment_id);

        // Verify creature1 has counter
        assert_eq!(
            game.object(creature1_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );

        // Move equipment to creature2
        {
            let equipment = game.object_mut(equipment_id).unwrap();
            equipment.attached_to = Some(creature2_id);
        }
        {
            let creature1 = game.object_mut(creature1_id).unwrap();
            creature1.attachments.retain(|&id| id != equipment_id);
        }
        {
            let creature2 = game.object_mut(creature2_id).unwrap();
            creature2.attachments.push(equipment_id);
        }

        // Trigger again - should put counter on creature2 now
        execute_blade_trigger(&mut game, alice, equipment_id);

        // Verify creature1 still has 1 counter (no change)
        assert_eq!(
            game.object(creature1_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );
        // Verify creature2 now has 1 counter
        assert_eq!(
            game.object(creature2_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );
    }

    // ========================================
    // Vampire with Multiple Subtypes Tests
    // ========================================

    #[test]
    fn test_vampire_knight_gets_two_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Vampire Knight (has multiple subtypes including Vampire)
        let creature_id = create_creature(
            &mut game,
            "Vampire Knight",
            vec![Subtype::Vampire, Subtype::Knight],
            alice,
        );
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Trigger the effect
        execute_blade_trigger(&mut game, alice, equipment_id);

        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2)
        );
    }

    // ========================================
    // Opponent's Creature Tests
    // ========================================

    #[test]
    fn test_triggers_when_opponent_creature_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice has the blade equipped to her creature
        let alice_creature = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, alice_creature);

        // Create Bob's creature (will "die" to trigger)
        let _bob_creature = create_creature(&mut game, "Goblin", vec![Subtype::Goblin], bob);

        // The trigger fires (simulating Bob's creature dying)
        // Note: In actual gameplay, the trigger system would detect the death
        // Here we just verify the effect works correctly when triggered
        execute_blade_trigger(&mut game, alice, equipment_id);

        let alice_creature_obj = game.object(alice_creature).unwrap();
        assert_eq!(
            alice_creature_obj
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );
    }

    // ========================================
    // Full Integration Scenario Test
    // ========================================

    #[test]
    fn test_full_scenario_vampire_tribal() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up: Alice has a Vampire equipped with Blade of the Bloodchief
        let vampire_id = create_creature(
            &mut game,
            "Vampire Noble",
            vec![Subtype::Vampire, Subtype::Noble],
            alice,
        );
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, vampire_id);

        // Initial state: 2/2 vampire with no counters
        {
            let vampire = game.object(vampire_id).unwrap();
            assert_eq!(vampire.power(), Some(2));
            assert_eq!(vampire.toughness(), Some(2));
            assert_eq!(vampire.counters.get(&CounterType::PlusOnePlusOne), None);
        }

        // Simulate 5 creature deaths (maybe from combat or sacrifice effects)
        for i in 1..=5 {
            execute_blade_trigger(&mut game, alice, equipment_id);

            // Verify counters after each death
            let vampire = game.object(vampire_id).unwrap();
            assert_eq!(
                vampire.counters.get(&CounterType::PlusOnePlusOne),
                Some(&(i * 2)),
                "Vampire should have {} counters after {} deaths",
                i * 2,
                i
            );
        }

        // Final state: 2/2 base + 10 counters = 12/12
        let vampire = game.object(vampire_id).unwrap();
        assert_eq!(vampire.power(), Some(12));
        assert_eq!(vampire.toughness(), Some(12));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_blade_of_the_bloodchief_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Island
                "2", // Tap Island for mana
                "1", // Cast Blade of the Bloodchief
            ],
            ReplayTestConfig::new().p1_hand(vec!["Blade of the Bloodchief", "Island"]),
        );

        assert!(
            game.battlefield_has("Blade of the Bloodchief"),
            "Blade of the Bloodchief should be on battlefield after casting"
        );
    }
}
