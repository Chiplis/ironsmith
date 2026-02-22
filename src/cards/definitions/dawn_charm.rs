//! Dawn Charm card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Dawn Charm - {1}{W}
/// Instant
/// Choose one —
/// • Prevent all combat damage that would be dealt this turn.
/// • Regenerate target creature.
/// • Counter target spell that targets you.
pub fn dawn_charm() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Dawn Charm")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one —\n\
            • Prevent all combat damage that would be dealt this turn.\n\
            • Regenerate target creature.\n\
            • Counter target spell that targets you.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EffectExecutor;
    use crate::ability::Ability;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::{Color, ColorSet};
    use crate::effect::EffectResult;
    use crate::effect::{Effect, Until};
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::StackEntry;
    use crate::game_state::{GameState, Target};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::zone::Zone;

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

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_dawn_charm_basic_properties() {
        let def = dawn_charm();
        assert_eq!(def.name(), "Dawn Charm");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_dawn_charm_is_instant() {
        let def = dawn_charm();
        assert!(def.card.is_instant());
    }

    #[test]
    fn test_dawn_charm_is_white() {
        let def = dawn_charm();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_dawn_charm_has_modal_effect() {
        let def = dawn_charm();
        assert!(def.spell_effect.is_some());
        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
        // The effect should be a ChooseModeEffect
        assert!(format!("{:?}", effects[0]).contains("ChooseModeEffect"));
    }

    // ========================================
    // Mode 1: Prevent Combat Damage Tests
    // ========================================

    #[test]
    fn test_dawn_charm_mode_1_prevents_combat_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Verify a prevention shield was added
        assert_eq!(game.prevention_effects.shields().len(), 1);

        // Verify the shield prevents combat damage
        let shield = &game.prevention_effects.shields()[0];
        assert!(
            shield.damage_filter.combat_only,
            "Shield should only prevent combat damage"
        );
    }

    #[test]
    fn test_dawn_charm_mode_1_prevents_combat_damage_to_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Execute the effect
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Try to apply combat damage
        let remaining = game.prevention_effects.apply_prevention_to_player(
            alice,
            5,
            true, // is combat
            ObjectId::from_raw(999),
            &ColorSet::COLORLESS,
            &vec![CardType::Creature],
            true,
        );

        assert_eq!(remaining, 0, "Combat damage should be prevented");
    }

    #[test]
    fn test_dawn_charm_mode_1_does_not_prevent_noncombat_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Execute the effect
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Try to apply noncombat damage
        let remaining = game.prevention_effects.apply_prevention_to_player(
            alice,
            5,
            false, // NOT combat
            ObjectId::from_raw(999),
            &ColorSet::RED,
            &vec![CardType::Instant],
            true,
        );

        assert_eq!(remaining, 5, "Noncombat damage should NOT be prevented");
    }

    // ========================================
    // Mode 2: Regenerate Tests
    // ========================================

    #[test]
    fn test_dawn_charm_mode_2_regenerates_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature
        let creature = create_creature(&mut game, "Soldier", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        // Use the regenerate effect directly
        let effect =
            crate::effects::RegenerateEffect::new(ChooseSpec::creature(), Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Creature should have 1 regeneration shield (one-shot replacement effect)
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature),
            1
        );
    }

    // ========================================
    // Mode 3: Counter Spell Targeting You Tests
    // ========================================

    #[test]
    fn test_dawn_charm_mode_3_counters_spell_targeting_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell targeting Alice
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Lightning Bolt",
            bob,
            vec![Target::Player(alice)],
        );

        // Alice uses mode 3
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = Effect::counter(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::spell().targeting_player(PlayerFilter::You),
        )));
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.stack.is_empty());
        // Spell should be in Bob's graveyard
        assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_dawn_charm_mode_3_fails_on_spell_targeting_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice has a creature
        let creature = create_creature(&mut game, "Soldier", alice);

        // Bob casts a spell targeting Alice's creature (not Alice herself)
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Doom Blade",
            bob,
            vec![Target::Object(creature)],
        );

        // Filter should reject spells not targeting you
        let filter = ObjectFilter::spell().targeting_player(PlayerFilter::You);
        let filter_ctx = game.filter_context_for(alice, None);
        let spell_obj = game.object(spell_id).unwrap();
        assert!(
            !filter.matches(spell_obj, &filter_ctx, &game),
            "Spell targeting a permanent should not match 'targets you' filter"
        );
    }

    #[test]
    fn test_dawn_charm_mode_3_fails_on_spell_targeting_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell targeting himself
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Healing Salve",
            bob,
            vec![Target::Player(bob)],
        );

        // Filter should reject spells not targeting you
        let filter = ObjectFilter::spell().targeting_player(PlayerFilter::You);
        let filter_ctx = game.filter_context_for(alice, None);
        let spell_obj = game.object(spell_id).unwrap();
        assert!(
            !filter.matches(spell_obj, &filter_ctx, &game),
            "Spell targeting opponent should not match 'targets you' filter"
        );
    }

    #[test]
    fn test_dawn_charm_mode_3_does_not_counter_uncounterable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts an uncounterable spell targeting Alice
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Abrupt Decay",
            bob,
            vec![Target::Player(alice)],
        );

        // Add "can't be countered" to the spell
        if let Some(obj) = game.object_mut(spell_id) {
            obj.abilities.push(Ability {
                kind: AbilityKind::Static(crate::static_abilities::StaticAbility::uncounterable()),
                functional_zones: vec![Zone::Stack],
                text: Some("can't be countered".to_string()),
            });
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = Effect::counter(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::spell().targeting_player(PlayerFilter::You),
        )));
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Spell can't be countered
        assert_eq!(result.result, EffectResult::Protected);
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_dawn_charm_mode_3_with_multiple_targets_including_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell targeting both Alice and himself
        let spell_id = create_spell_on_stack_targeting(
            &mut game,
            "Arc Trail",
            bob,
            vec![Target::Player(alice), Target::Player(bob)],
        );

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = Effect::counter(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::spell().targeting_player(PlayerFilter::You),
        )));
        let result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Should work - one of the targets is Alice
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.stack.is_empty());
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_dawn_charm_oracle_text() {
        let def = dawn_charm();
        assert!(def.card.oracle_text.contains("Choose one"));
        assert!(def.card.oracle_text.contains("Prevent all combat damage"));
        assert!(def.card.oracle_text.contains("Regenerate target creature"));
        assert!(
            def.card
                .oracle_text
                .contains("Counter target spell that targets you")
        );
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_dawn_charm_casting() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Dawn Charm
                "0", // Choose mode (first mode - prevent combat damage)
                "0", // Tap first Plains
                "0", // Tap second Plains
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Dawn Charm"])
                .p1_battlefield(vec!["Plains", "Plains"]),
        );

        // Dawn Charm is an instant, should be in graveyard after resolving
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        let in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Dawn Charm")
                .unwrap_or(false)
        });
        assert!(
            in_graveyard,
            "Dawn Charm should be in graveyard after resolving"
        );
    }
}
