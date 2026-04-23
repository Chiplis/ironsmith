//! Player decision system for MTG.
//!
//! This module provides:
//! - `DecisionMaker` and typed decision contexts for player input
//! - `LegalAction` and related types for describing legal game actions
//! - Helper functions to compute legal actions

use crate::alternative_cast::CastingMethod;
use crate::combat_state::{AttackTarget, CombatState};
use crate::derived_view::DerivedGameView;
use crate::effects::ExecutionContext;
use crate::effects::helpers::resolve_value;
use crate::game_state::{GameState, Phase, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::perf::PerfTimer;
use crate::special_actions::{SpecialAction, can_activate_mana_ability_check_with_view};
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;
use crate::zone::Zone;
use crate::{CounterType, ManaSymbol, Step};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

mod attack_block;
mod io;
mod legal_actions;
mod mana;
mod perf;
mod types;

#[allow(unused_imports)]
use attack_block::*;
#[allow(unused_imports)]
use io::*;
#[allow(unused_imports)]
use legal_actions::*;
#[allow(unused_imports)]
use mana::*;
#[allow(unused_imports)]
use perf::*;
#[allow(unused_imports)]
use types::*;

pub use attack_block::*;
pub use io::*;
pub use legal_actions::*;
pub use mana::*;
pub use perf::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinitionBuilder;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::{basic_island, counterspell, force_of_will, lightning_bolt};
    use crate::color::ColorSet;
    use crate::costs::PaymentReason;
    use crate::decisions::context::{TargetRequirementContext, TargetsContext};
    use crate::effect::{Effect, Value};
    use crate::filter::Comparison;
    use crate::grant::Grantable;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn stage_spell_cast_for_test(
        game: &mut GameState,
        spell_id: ObjectId,
        caster: PlayerId,
        from_zone: Zone,
    ) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell_id, caster, from_zone),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_cards_drawn_for_test(game: &mut GameState, player: PlayerId, count: u32) {
        let cards = (0..count).map(|_| game.new_object_id()).collect();
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CardsDrawnEvent::new(player, cards, count > 0),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_commit_crime_for_test(game: &mut GameState, player: PlayerId) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::KeywordActionEvent::new(
                crate::events::other::KeywordActionKind::CommitCrime,
                player,
                ObjectId::from_raw(0),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_life_gain_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(player, amount),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_life_loss_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeLossEvent::from_effect(player, amount),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn krrik_keeps_black_spell_costs_as_black_pips() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = CardBuilder::new(CardId::from_raw(7000), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("helper permanent should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7001), "Black Cost Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{1}{B}{B}");
    }

    #[test]
    fn trinisphere_raises_single_black_spell_to_three_total_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = CardBuilder::new(CardId::from_raw(7002), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("helper permanent should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7003), "Cheap Black Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{2}");
        assert_eq!(effective.mana_value(), 3);
    }

    #[test]
    fn trinisphere_counts_krrik_life_paid_black_pips_toward_floor() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let krrik = CardBuilder::new(CardId::from_raw(7004), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
        game.object_mut(krrik_id)
            .expect("krrik helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let trini = CardBuilder::new(CardId::from_raw(7005), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
        game.object_mut(trini_id)
            .expect("trinisphere helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7006), "Necro Probe")
            .card_types(vec![CardType::Enchantment])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Black,
                ManaSymbol::Black,
                ManaSymbol::Black,
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let effective = {
            let spell_obj = game.object(spell_id).expect("spell should exist");
            let base_cost = spell_obj
                .mana_cost
                .as_ref()
                .expect("spell should have a cost");
            calculate_effective_mana_cost(&game, alice, spell_obj, base_cost)
        };

        assert_eq!(effective.to_oracle(), "{B}{B}{B}");
        assert_eq!(effective.mana_value(), 3);
        assert!(
            game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell_id),
                &effective,
                0,
                PaymentReason::CastSpell
            ),
            "three black pips should already satisfy Trinisphere even when Krrik pays them with life"
        );
        assert_eq!(game.player(alice).expect("alice exists").life, 14);
    }

    #[test]
    fn yasharn_blocks_krrik_life_payment_without_rewriting_spell_costs() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let krrik = CardBuilder::new(CardId::from_raw(7007), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
        game.object_mut(krrik_id)
            .expect("krrik helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let yasharn = CardBuilder::new(CardId::from_raw(7008), "Yasharn Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
        game.object_mut(yasharn_id)
            .expect("yasharn helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7009), "Yasharn Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Black,
                ManaSymbol::Black,
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{B}");
        assert!(
            !game.can_pay_mana_cost_with_reason(
                alice,
                Some(spell_id),
                &effective,
                0,
                PaymentReason::CastSpell
            ),
            "without black mana in the pool, Yasharn should remove Krrik's life-payment option"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn yasharn_blocks_force_of_will_alternative_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
        game.stack.push(crate::StackEntry::new(bolt_id, bob));

        let yasharn = CardBuilder::new(CardId::from_raw(7010), "Yasharn Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
        game.object_mut(yasharn_id)
            .expect("yasharn helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ));

        let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
        game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "Yasharn should stop Force of Will's alternative cost because it includes paying life"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn trinisphere_requires_three_mana_for_force_of_will_alternative_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
        game.stack.push(crate::StackEntry::new(bolt_id, bob));

        let trini = CardBuilder::new(CardId::from_raw(7011), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
        game.object_mut(trini_id)
            .expect("trinisphere helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
        game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

        for _ in 0..2 {
            game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
        }
        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "Trinisphere should make Force of Will's free alternative cost require three mana"
        );

        game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "with three Islands available, the alternative cost should become legal again"
        );
    }

    fn stage_noncombat_damage_to_player_for_test(
        game: &mut GameState,
        source: ObjectId,
        player: PlayerId,
        amount: u32,
    ) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(player),
                amount,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_artifact_sacrifice_for_test(game: &mut GameState, player: PlayerId) {
        let artifact = CardBuilder::new(CardId::new(), "Sacrificed Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, player, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(artifact_id).expect("artifact exists"),
            game,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::permanents::SacrificeEvent::new(artifact_id, None)
                .with_snapshot(Some(snapshot), Some(player)),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn test_compute_legal_actions_basic() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let actions = compute_legal_actions(&game, alice);

        // Should at least have pass priority
        assert!(actions.contains(&LegalAction::PassPriority));
    }

    #[test]
    fn test_compute_legal_actions_with_land() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Add a land to hand
        let land = CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);

        // Should have play land action
        assert!(actions.contains(&LegalAction::PlayLand { land_id }));
    }

    #[test]
    fn test_compute_legal_actions_includes_graveyard_land_with_play_from_grant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_018), "Ash Barrens")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.effect_store
            .grant_registry
            .grant_to_filter_until_end_of_turn(
                ObjectFilter::default().with_type(CardType::Land),
                Zone::Graveyard,
                alice,
                Grantable::play_from(),
                source_id,
                game.turn.turn_number,
            );

        let actions = compute_legal_actions(&game, alice);

        assert!(
            actions.contains(&LegalAction::PlayLand { land_id }),
            "play-from-graveyard grants should surface playable lands as land actions"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_graveyard_land_after_land_play_used() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_019), "Haunted Mire")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.effect_store
            .grant_registry
            .grant_to_filter_until_end_of_turn(
                ObjectFilter::default().with_type(CardType::Land),
                Zone::Graveyard,
                alice,
                Grantable::play_from(),
                source_id,
                game.turn.turn_number,
            );

        game.player_mut(alice)
            .expect("alice should exist")
            .record_land_play();

        let actions = compute_legal_actions(&game, alice);

        assert!(
            !actions.contains(&LegalAction::PlayLand { land_id }),
            "granted graveyard land plays must still respect the per-turn land limit"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_exile_land_with_play_from_grant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_020), "Forgotten Cave")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Exile);

        let source_id = game.new_object_id();
        game.effect_store
            .grant_registry
            .grant_to_filter_until_end_of_turn(
                ObjectFilter::default().with_type(CardType::Land),
                Zone::Exile,
                alice,
                Grantable::play_from(),
                source_id,
                game.turn.turn_number,
            );

        let actions = compute_legal_actions(&game, alice);

        assert!(
            actions.contains(&LegalAction::PlayLand { land_id }),
            "public-zone play-from grants should continue to surface exile lands"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn simple_battlefield_mana_ability_output_recognizes_basic_land_tap() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let forest = CardDefinitionBuilder::new(CardId::from_raw(700_901), "Forest")
            .card_types(vec![CardType::Land])
            .parse_text("{T}: Add {G}.")
            .expect("forest mana text should parse");
        let forest_id = game.create_object_from_definition(&forest, alice, Zone::Battlefield);
        let ability = game
            .current_ability(forest_id, 0)
            .expect("forest should expose a mana ability");
        let view = DerivedGameView::new(&game);

        assert_eq!(
            simple_battlefield_mana_ability_output(&game, alice, forest_id, 0, &ability, &view),
            Some(vec![ManaSymbol::Green]),
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn simple_battlefield_mana_ability_output_ignores_non_mana_activated_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let yawgmoth = crate::cards::definitions::yawgmoth_thran_physician();
        let yawgmoth_id = game.create_object_from_definition(&yawgmoth, alice, Zone::Battlefield);
        let ability = game
            .current_ability(yawgmoth_id, 0)
            .expect("Yawgmoth should expose its first activated ability");
        let view = DerivedGameView::new(&game);

        assert_eq!(
            simple_battlefield_mana_ability_output(&game, alice, yawgmoth_id, 0, &ability, &view),
            None,
        );
    }

    #[test]
    fn test_select_first_decision_maker_supports_multi_target_requirement() {
        let first = Target::Object(ObjectId::from_raw(1));
        let second = Target::Object(ObjectId::from_raw(2));
        let ctx = TargetsContext::new(
            PlayerId::from_index(0),
            ObjectId::from_raw(99),
            "test spell",
            vec![TargetRequirementContext {
                description: "two targets".to_string(),
                legal_targets: vec![first, second],
                min_targets: 2,
                max_targets: Some(2),
            }],
        );

        let mut dm = SelectFirstDecisionMaker;
        let chosen = dm.decide_targets(&setup_game(), &ctx);

        assert_eq!(chosen, vec![first, second]);
    }

    /// Tests computation of legal attackers during declare attackers step.
    ///
    /// Scenario: Alice controls a Grizzly Bears that has been on the battlefield
    /// since the beginning of her turn (no summoning sickness). When computing
    /// legal attackers, it should be available to attack Bob (player 1).
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_attackers() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Grizzly Bears on battlefield
        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

        // Remove summoning sickness (creature has been on battlefield since turn start)
        game.remove_summoning_sickness(creature_id);

        let combat = CombatState::default();
        let options = compute_legal_attackers(&game, &combat);

        assert_eq!(options.len(), 1, "Should have one legal attacker");
        assert_eq!(options[0].creature, creature_id);
        assert!(
            !options[0].must_attack,
            "Grizzly Bears doesn't have 'must attack'"
        );
        // Should be able to attack Bob (player 1)
        assert!(
            options[0]
                .valid_targets
                .contains(&AttackTarget::Player(bob)),
            "Should be able to attack the opponent"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_attackers_respects_cant_attack_restriction_tracker() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);
        game.effect_store
            .cant_effects
            .cant_attack
            .insert(creature_id);

        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.is_empty(),
            "cant-attack tracker should prevent declaring attackers, got {options:?}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_attackers_respects_cant_attack_alone_with_single_attacker() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);
        game.effect_store
            .cant_effects
            .cant_attack_alone
            .insert(creature_id);

        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.is_empty(),
            "single creature with can't-attack-alone should not be legal attacker, got {options:?}"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_cast_creature_spell_attack_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let cohort_card = CardBuilder::new(CardId::from_raw(901), "Goblin Cohort Variant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(
                crate::card::PtValue::Fixed(2),
                crate::card::PtValue::Fixed(2),
            ))
            .build();
        let cohort_id = game.create_object_from_card(&cohort_card, alice, Zone::Battlefield);
        game.object_mut(cohort_id)
            .expect("cohort exists")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
            ));
        game.remove_summoning_sickness(cohort_id);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().all(|option| option.creature != cohort_id),
            "cohort should not be legal attacker before controller casts a creature spell this turn"
        );

        let prior_creature = CardBuilder::new(CardId::from_raw(902), "Prior Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let prior_id = game.create_object_from_card(&prior_creature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_id).expect("prior creature exists"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().any(|option| option.creature == cohort_id),
            "cohort should become a legal attacker after controller casts a creature spell this turn"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_graveyard_threshold_attack_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let threshold_card = CardBuilder::new(CardId::from_raw(903), "Threshold Raider Variant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(
                crate::card::PtValue::Fixed(3),
                crate::card::PtValue::Fixed(2),
            ))
            .build();
        let attacker_id = game.create_object_from_card(&threshold_card, alice, Zone::Battlefield);
        game.object_mut(attacker_id)
            .expect("threshold attacker exists")
                .abilities
                .push(Ability::static_ability(
                    StaticAbility::cant_attack_unless_condition(
                    crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(5),
                    "Can't attack unless there are five or more cards in your graveyard",
                ),
            ));
        game.remove_summoning_sickness(attacker_id);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().all(|option| option.creature != attacker_id),
            "attacker should not be legal before threshold is met"
        );

        for idx in 0..5 {
            let filler =
                CardBuilder::new(CardId::from_raw(1000 + idx), &format!("Filler {}", idx + 1))
                    .card_types(vec![CardType::Creature])
                    .build();
            let _ = game.create_object_from_card(&filler, alice, Zone::Graveyard);
        }

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().any(|option| option.creature == attacker_id),
            "attacker should become legal after graveyard threshold is met"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_blockers_respects_cant_block_alone_with_single_blocker() {
        use crate::cards::definitions::grizzly_bears;
        use crate::combat_state::AttackerInfo;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let blocker_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
        game.effect_store
            .cant_effects
            .cant_block_alone
            .insert(blocker_id);

        let attacker_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });

        let options = compute_legal_blockers(&game, &combat, alice);
        assert_eq!(options.len(), 1, "expected one attacker option");
        assert!(
            options[0].valid_blockers.is_empty(),
            "single creature with can't-block-alone should not be a legal blocker, got {options:?}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_blockers_excludes_tapped_creatures() {
        use crate::cards::definitions::grizzly_bears;
        use crate::combat_state::AttackerInfo;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let blocker_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
        game.tap(blocker_id);

        let attacker_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });

        let options = compute_legal_blockers(&game, &combat, alice);
        assert_eq!(options.len(), 1, "expected one attacker option");
        assert!(
            options[0].valid_blockers.is_empty(),
            "tapped creature should not be a legal blocker, got {options:?}"
        );
    }

    #[test]
    fn global_colored_spell_cost_increase_adds_pips_to_effective_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Battlefield permanent that taxes black spells you cast by {B}.
        let tax_card = CardBuilder::new(CardId::from_raw(10), "Derelor Variant")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.colors = Some(ColorSet::BLACK);
        filter.cast_by = Some(PlayerFilter::You);
        let tax = StaticAbility::new(crate::static_abilities::CostIncreaseManaCost::new(
            filter,
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        // A black spell with base cost {1}{B}.
        let black_spell_card = CardBuilder::new(CardId::from_raw(11), "Black Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&black_spell_card, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{1}{B}{B}");
    }

    #[test]
    fn global_spell_cost_increase_matches_spell_filter_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let tax_card = CardBuilder::new(CardId::from_raw(12), "Power Tax")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.power = Some(Comparison::GreaterThanOrEqual(4));
        let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
            filter,
            Value::Fixed(1),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        let creature_spell = CardBuilder::new(CardId::from_raw(13), "Large Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&creature_spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{G}{1}");
    }

    #[test]
    fn global_spell_cost_increase_uses_caster_for_spell_filter_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let tax_card = CardBuilder::new(CardId::from_raw(14), "Caster Tax")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);
        let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
            filter,
            Value::Fixed(1),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        let spell_card = CardBuilder::new(CardId::from_raw(15), "Borrowed Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        // Bob owns/controls the card object, but we evaluate castability for Alice.
        let spell_id = game.create_object_from_card(&spell_card, bob, Zone::Exile);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective_for_alice = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective_for_alice.to_oracle(), "{2}{U}{1}");

        let effective_for_bob = calculate_effective_mana_cost(&game, bob, spell_obj, base_cost);
        assert_eq!(effective_for_bob.to_oracle(), "{2}{U}");
    }

    #[test]
    fn spell_attached_global_cost_reduction_requires_functional_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(16), "Zone Scoped Reducer")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .build();
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);

        // A battlefield-only static modifier on a spell card in hand should not apply.
        let battlefield_only_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            filter.clone(),
            Value::Fixed(1),
        ));
        game.object_mut(battlefield_only_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction));
        let battlefield_only_obj = game.object(battlefield_only_id).expect("spell exists");
        let battlefield_only_base = battlefield_only_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let battlefield_only_effective = calculate_effective_mana_cost(
            &game,
            alice,
            battlefield_only_obj,
            battlefield_only_base,
        );
        assert_eq!(
            battlefield_only_effective.to_oracle(),
            "{2}",
            "battlefield-only modifiers must not apply while the spell is in hand"
        );

        // A hand/stack-scoped modifier still applies (e.g. Undaunted-style implementations).
        let hand_scoped_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let hand_scoped_reduction = StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, Value::Fixed(1)),
        );
        game.object_mut(hand_scoped_id)
            .expect("spell exists")
            .abilities
            .push(
                Ability::static_ability(hand_scoped_reduction)
                    .in_zones(vec![Zone::Hand, Zone::Stack]),
            );
        let hand_scoped_obj = game.object(hand_scoped_id).expect("spell exists");
        let hand_scoped_base = hand_scoped_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let hand_scoped_effective =
            calculate_effective_mana_cost(&game, alice, hand_scoped_obj, hand_scoped_base);
        assert_eq!(
            hand_scoped_effective.to_oracle(),
            "{1}",
            "zone-scoped spell modifiers should still apply"
        );
    }

    #[test]
    fn spell_attached_global_cost_reduction_respects_color_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mut red_filter = ObjectFilter::default();
        red_filter.cast_by = Some(PlayerFilter::You);
        red_filter.colors = Some(ColorSet::RED);

        let colorless_spell = CardBuilder::new(CardId::from_raw(17), "Colorless Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .build();
        let colorless_id = game.create_object_from_card(&colorless_spell, alice, Zone::Hand);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            red_filter.clone(),
            Value::Fixed(1),
        ));
        game.object_mut(colorless_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
        let colorless_obj = game.object(colorless_id).expect("spell exists");
        let colorless_base = colorless_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let colorless_effective =
            calculate_effective_mana_cost(&game, alice, colorless_obj, colorless_base);
        assert_eq!(
            colorless_effective.to_oracle(),
            "{2}",
            "red-only filter must not reduce non-red spell costs"
        );

        let red_spell = CardBuilder::new(CardId::from_raw(18), "Red Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let red_id = game.create_object_from_card(&red_spell, alice, Zone::Hand);
        let red_reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            red_filter,
            Value::Fixed(1),
        ));
        game.object_mut(red_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(red_reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
        let red_obj = game.object(red_id).expect("spell exists");
        let red_base = red_obj.mana_cost.as_ref().expect("spell has mana cost");
        let red_effective = calculate_effective_mana_cost(&game, alice, red_obj, red_base);
        assert_eq!(
            red_effective.to_oracle(),
            "{R}",
            "red-only filter should reduce matching red spell costs"
        );
    }

    #[test]
    fn dynamic_spell_cost_reduction_distinct_names_reduces_generic_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Two differently named lands you control.
        let forest = CardBuilder::new(CardId::from_raw(20), "Forest Variant")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&forest, alice, Zone::Battlefield);
        let island = CardBuilder::new(CardId::from_raw(21), "Island Variant")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&island, alice, Zone::Battlefield);

        // A spell with base cost {6}{G} that costs {X} less where X is distinct land names.
        let spell_card = CardBuilder::new(CardId::from_raw(22), "Fungal Colossus Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let mut filter = ObjectFilter::land().you_control();
        filter.zone = Some(Zone::Battlefield);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            ObjectFilter::default(),
            Value::DistinctNames(filter),
        ));

        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{G}");
    }

    #[test]
    fn conditional_this_spell_cost_reduction_only_applies_when_active() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(30), "Avatar Cost Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(6),
            crate::static_abilities::ThisSpellCostCondition::YouLifeTotalOrLess(3),
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        game.player_mut(alice).expect("alice exists").life = 4;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{B}{B}");

        // Condition met.
        game.player_mut(alice).expect("alice exists").life = 3;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_counts_distinct_creature_types_with_cap() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        for (idx, subtypes) in [
            vec![Subtype::Elf, Subtype::Druid],
            vec![Subtype::Goblin, Subtype::Warrior],
            vec![Subtype::Human, Subtype::Soldier],
        ]
        .into_iter()
        .enumerate()
        {
            let creature = CardBuilder::new(
                CardId::from_raw(100 + idx as u32),
                format!("Type Bearer {idx}"),
            )
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
            game.create_object_from_card(&creature, alice, Zone::Battlefield);
        }

        let spell_card = CardBuilder::new(CardId::from_raw(140), "Capped Type Discount")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(7)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let amount = Value::Min(
            Box::new(Value::CreatureTypesAmong(
                ObjectFilter::creature().you_control(),
            )),
            Box::new(Value::Fixed(5)),
        );
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            amount,
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{W}{W}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_checks_opponent_drawn_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(31), "Even the Score Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(4),
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        stage_cards_drawn_for_test(&mut game, bob, 3);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{U}{U}{U}");

        // Condition met.
        game.turn_store.turn_history.clear_for_new_turn();
        stage_cards_drawn_for_test(&mut game, bob, 4);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_checks_opponent_cast_spells() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(32), "Ertai's Scorn Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(
                    2,
                ),
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");

        // Condition met.
        game.turn_store.turn_history.clear_for_new_turn();
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3202), bob, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_with_generic_and_colored_pips() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(33), "Discontinuity Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::YourTurn,
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition met (it's your turn).
        game.turn.active_player = alice;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}");

        // Condition not met.
        game.turn.active_player = bob;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{U}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_with_target_condition_uses_chosen_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(133), "Target Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::TargetsObject(
            ObjectFilter::creature().tapped(),
        );
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let creature_card = CardBuilder::new(CardId::from_raw(134), "Target Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        // Untapped target does not satisfy condition.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
            &game,
            alice,
            spell_obj,
            base_cost,
            &[Target::Object(creature_id)],
        );
        assert_eq!(effective.to_oracle(), "{3}{R}");

        // Tapped target satisfies condition.
        game.tap(creature_id);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
            &game,
            alice,
            spell_obj,
            base_cost,
            &[Target::Object(creature_id)],
        );
        assert_eq!(effective.to_oracle(), "{1}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_cast_another_instant_or_sorcery_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(135), "Spell History Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: vec![CardType::Instant, CardType::Sorcery],
            };
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // No prior instant/sorcery this turn.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{U}");

        // One instant cast this turn enables reduction.
        let prior_card = CardBuilder::new(CardId::from_raw(136), "Prior Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_id = game.create_object_from_card(&prior_card, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_id).expect("prior instant exists"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_graveyard_card_count_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card =
            CardBuilder::new(CardId::from_raw(137), "Graveyard Cards Discount Variant")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(8)],
                    vec![ManaSymbol::Black],
                ]))
                .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(9);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(3),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Not enough cards.
        for idx in 0..8 {
            let filler = CardBuilder::new(CardId::from_raw(200 + idx), format!("GY Card {idx}"))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&filler, alice, Zone::Graveyard);
        }
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{8}{B}");

        // Ninth card enables reduction.
        let extra = CardBuilder::new(CardId::from_raw(300), "GY Extra")
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&extra, alice, Zone::Graveyard);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_creature_attacking_you_condition() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(138), "Attack Trap Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::CreatureIsAttackingYou;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // No attackers: no reduction.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{B}");

        // One attacker attacking Alice enables reduction.
        let attacker_card = CardBuilder::new(CardId::from_raw(139), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let attacker_id = game.create_object_from_card(&attacker_card, bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });
        game.combat = Some(combat);

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_is_night_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(140), "Night Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::IsNight;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{R}");

        game.is_night = true;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_sacrificed_artifact_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(141), "Artifact Sac Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(5)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouSacrificedArtifactThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(3),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{R}");

        stage_artifact_sacrifice_for_test(&mut game, alice);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_creature_left_battlefield_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(142), "Creature Left Discount Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::
            CreatureLeftBattlefieldUnderYourControlThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{G}");

        let departed_creature = CardBuilder::new(CardId::from_raw(5000), "Fallen Helper")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let departed_id =
            game.create_object_from_card(&departed_creature, alice, Zone::Battlefield);
        game.move_object_by_effect(departed_id, Zone::Graveyard);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_committed_crime_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(143), "Crime Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::YouCommittedCrimeThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(1),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{U}");

        stage_commit_crime_for_test(&mut game, alice);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_only_named_creatures_in_hand_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(144), "Mothrider Cavalry")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::White],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(
                "mothrider cavalry".to_string(),
            );
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Only this card in hand (named Mothrider Cavalry): reduction applies.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{W}");

        // Another creature with a different name disables the reduction.
        let other_creature = CardBuilder::new(CardId::from_raw(145), "Not Mothrider")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&other_creature, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{W}");
    }

    #[test]
    fn this_spell_cost_reduction_x_uses_life_difference_from_starting() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card =
            CardBuilder::new(CardId::from_raw(146), "Starting Life X Discount Variant")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(13)],
                    vec![ManaSymbol::Black],
                ]))
                .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::X,
            crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // At starting life, no reduction.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{13}{B}");

        // Reduced by life lost from starting life total.
        game.player_mut(alice).expect("player exists").life = 12;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_devotion_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Devotion to black = 3.
        let perm1 = CardBuilder::new(CardId::from_raw(40), "BB Permanent")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        game.create_object_from_card(&perm1, alice, Zone::Battlefield);
        let perm2 = CardBuilder::new(CardId::from_raw(41), "1B Permanent")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        game.create_object_from_card(&perm2, alice, Zone::Battlefield);

        let spell_card = CardBuilder::new(CardId::from_raw(42), "Devotion Cost Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Black,
            },
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_total_power_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bear = CardBuilder::new(CardId::from_raw(43), "Cost Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        game.create_object_from_card(&bear, alice, Zone::Battlefield);
        let giant = CardBuilder::new(CardId::from_raw(44), "Cost Giant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        game.create_object_from_card(&giant, alice, Zone::Battlefield);

        let spell_card = CardBuilder::new(CardId::from_raw(45), "Power Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(10)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::TotalPower(ObjectFilter::creature().you_control()),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_life_gained_this_turn_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        stage_life_gain_for_test(&mut game, alice, 5);

        let spell_card = CardBuilder::new(CardId::from_raw(46), "Life Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(7)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::LifeGainedThisTurn(PlayerFilter::You),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_noncombat_damage_to_opponents_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        stage_noncombat_damage_to_player_for_test(&mut game, ObjectId::from_raw(4701), bob, 6);

        let spell_card = CardBuilder::new(CardId::from_raw(47), "Damage Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(8)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::NoncombatDamageDealtToPlayersThisTurn(PlayerFilter::Opponent),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_greatest_commander_mana_value_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let commander_battlefield = CardBuilder::new(CardId::from_raw(48), "Battlefield Commander")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green],
            ]))
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let battlefield_id =
            game.create_object_from_card(&commander_battlefield, alice, Zone::Battlefield);
        game.set_as_commander(battlefield_id, alice);

        let commander_command_zone =
            CardBuilder::new(CardId::from_raw(49), "Command Zone Commander")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(5)],
                    vec![ManaSymbol::Blue],
                ]))
                .power_toughness(PowerToughness::fixed(5, 5))
                .build();
        let command_id =
            game.create_object_from_card(&commander_command_zone, alice, Zone::Command);
        game.set_as_commander(command_id, alice);

        let spell_card = CardBuilder::new(CardId::from_raw(50), "Commander Discount Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(9)],
                vec![ManaSymbol::White],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);

        let mut battlefield_filter = ObjectFilter::default();
        battlefield_filter.zone = Some(Zone::Battlefield);
        battlefield_filter.owner = Some(PlayerFilter::You);
        battlefield_filter.is_commander = true;
        let mut command_filter = battlefield_filter.clone();
        command_filter.zone = Some(Zone::Command);
        let mut commander_filter = ObjectFilter::default();
        commander_filter.any_of = vec![battlefield_filter, command_filter];

        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::GreatestManaValue(commander_filter),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{W}");
    }

    #[test]
    fn test_can_cast_spell_respects_cant_cast_creature_spells_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(77), "Restriction Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        let creature_obj = game
            .object(creature_id)
            .expect("creature in hand must exist")
            .clone();

        game.effect_store.cant_effects.add_cant_cast_filter(
            alice,
            crate::target::ObjectFilter::default().with_type(CardType::Creature),
        );
        assert!(
            !can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
            "creature spell should be uncastable when player can't cast creature spells"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_cast_limit_one_per_turn_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let instant = CardBuilder::new(CardId::from_raw(78), "Restriction Spark")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
        let instant_obj = game
            .object(instant_id)
            .expect("instant in hand must exist")
            .clone();

        game.effect_store
            .cant_effects
            .add_cast_limit_filter(alice, crate::target::ObjectFilter::default());
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(7801), alice, Zone::Hand);

        assert!(
            !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
            "second spell in same turn should be blocked by one-spell limit"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_noncreature_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let instant = CardBuilder::new(CardId::from_raw(79), "Restriction Snuff")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
        let instant_obj = game
            .object(instant_id)
            .expect("instant in hand must exist")
            .clone();

        let prior_noncreature = CardBuilder::new(CardId::from_raw(80), "Prior Noncreature")
            .card_types(vec![CardType::Sorcery])
            .build();
        let prior_noncreature_id =
            game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_noncreature_id)
                .expect("prior noncreature must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Creature),
        );

        assert!(
            !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
            "second noncreature spell in same turn should be blocked by noncreature cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_noncreature_limit_still_allows_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(81), "Restriction Beast")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        let creature_obj = game
            .object(creature_id)
            .expect("creature in hand must exist")
            .clone();

        let prior_noncreature = CardBuilder::new(CardId::from_raw(82), "Prior Noncreature")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_noncreature_id =
            game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_noncreature_id)
                .expect("prior noncreature must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Creature),
        );

        assert!(
            can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
            "noncreature cast limit should still allow creature spell"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_nonartifact_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let nonartifact_spell = CardBuilder::new(CardId::from_raw(83), "Restriction Chant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::new())
            .build();
        let nonartifact_spell_id =
            game.create_object_from_card(&nonartifact_spell, alice, Zone::Hand);
        let nonartifact_spell_obj = game
            .object(nonartifact_spell_id)
            .expect("nonartifact spell in hand must exist")
            .clone();

        let prior_nonartifact = CardBuilder::new(CardId::from_raw(84), "Prior Nonartifact")
            .card_types(vec![CardType::Sorcery])
            .build();
        let prior_nonartifact_id =
            game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonartifact_id)
                .expect("prior nonartifact must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Artifact),
        );

        assert!(
            !can_cast_spell(&game, alice, &nonartifact_spell_obj, &CastingMethod::Normal),
            "second nonartifact spell in same turn should be blocked by nonartifact cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_nonartifact_limit_allows_artifact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let artifact_spell = CardBuilder::new(CardId::from_raw(85), "Restriction Relic")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::new())
            .build();
        let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
        let artifact_spell_obj = game
            .object(artifact_spell_id)
            .expect("artifact spell in hand must exist")
            .clone();

        let prior_nonartifact = CardBuilder::new(CardId::from_raw(86), "Prior Nonartifact")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_nonartifact_id =
            game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonartifact_id)
                .expect("prior nonartifact must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Artifact),
        );

        assert!(
            can_cast_spell(&game, alice, &artifact_spell_obj, &CastingMethod::Normal),
            "nonartifact cast limit should still allow artifact spell"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_nonphyrexian_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let nonphyrexian_spell = CardBuilder::new(CardId::from_raw(87), "Restriction Spell")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let nonphyrexian_spell_id =
            game.create_object_from_card(&nonphyrexian_spell, alice, Zone::Hand);
        let nonphyrexian_spell_obj = game
            .object(nonphyrexian_spell_id)
            .expect("non-Phyrexian spell in hand must exist")
            .clone();

        let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(88), "Prior Nonphyrexian")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let prior_nonphyrexian_id =
            game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonphyrexian_id)
                .expect("prior non-Phyrexian must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
        );

        assert!(
            !can_cast_spell(
                &game,
                alice,
                &nonphyrexian_spell_obj,
                &CastingMethod::Normal
            ),
            "second non-Phyrexian spell in same turn should be blocked by non-Phyrexian cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_nonphyrexian_limit_allows_phyrexian() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let phyrexian_spell = CardBuilder::new(CardId::from_raw(89), "Restriction Horror")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .subtypes(vec![Subtype::Phyrexian])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let phyrexian_spell_id = game.create_object_from_card(&phyrexian_spell, alice, Zone::Hand);
        let phyrexian_spell_obj = game
            .object(phyrexian_spell_id)
            .expect("Phyrexian spell in hand must exist")
            .clone();

        let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(90), "Prior Nonphyrexian")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let prior_nonphyrexian_id =
            game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonphyrexian_id)
                .expect("prior non-Phyrexian must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.effect_store.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
        );

        assert!(
            can_cast_spell(&game, alice, &phyrexian_spell_obj, &CastingMethod::Normal),
            "non-Phyrexian cast limit should still allow a Phyrexian spell"
        );
    }

    #[test]
    fn test_can_cast_spell_uses_conditional_spell_flash_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.active_player = bob;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::BeginCombat);

        let sorcery = CardBuilder::new(CardId::from_raw(1200), "Threshold Flash Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::new())
            .build();
        let spell_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
        let spec = crate::static_abilities::ConditionalSpellKeywordSpec {
            keyword: crate::static_abilities::ConditionalSpellKeywordKind::Flash,
            metric: crate::static_abilities::GraveyardCountMetric::ManaValues,
            threshold: 5,
        };
        game.object_mut(spell_id)
            .expect("spell should exist")
            .abilities
            .push(
                Ability::static_ability(StaticAbility::conditional_spell_keyword(spec))
                    .in_zones(vec![Zone::Hand, Zone::Stack]),
            );

        for (idx, mv) in [1u8, 2, 3, 4].into_iter().enumerate() {
            let card = CardBuilder::new(
                CardId::from_raw(1300 + idx as u32),
                &format!("MV{mv} Graveyard Card"),
            )
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
            .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            !can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "sorcery should remain sorcery-speed before mana-value threshold is met"
        );

        let fifth = CardBuilder::new(CardId::from_raw(1399), "MV5 Graveyard Card")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
            .build();
        game.create_object_from_card(&fifth, alice, Zone::Graveyard);

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "conditional flash should allow casting once the mana-value threshold is met"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_actions_includes_kentaro_mana_value_cast_for_samurai() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let kentaro = CardDefinitionBuilder::new(CardId::from_raw(1400), "Kentaro Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Bushido 1\nYou may pay {X} rather than pay the mana cost for Samurai spells you cast, where X is that spell's mana value.",
            )
            .expect("Kentaro text should parse");
        let _kentaro_id = game.create_object_from_definition(&kentaro, alice, Zone::Battlefield);

        let samurai = CardBuilder::new(CardId::from_raw(1401), "Samurai Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Samurai])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::White],
            ]))
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let samurai_id = game.create_object_from_card(&samurai, alice, Zone::Hand);

        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .add(ManaSymbol::Colorless, 5);

        let granted = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, samurai_id, Zone::Hand, alice);
        assert_eq!(
            granted.len(),
            1,
            "Kentaro should grant one hand alternative cost"
        );
        assert_eq!(granted[0].method.name(), "Pay mana value");
        assert_eq!(
            granted[0]
                .method
                .mana_cost()
                .expect("Kentaro grant should have a mana cost")
                .generic_mana_total(),
            5,
            "Kentaro should turn the spell's mana value into a generic hand-cast cost"
        );

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == samurai_id
            )),
            "without white mana, the Samurai should not be normally castable"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Hand,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == samurai_id
            )),
            "Kentaro should surface a hand cast action that uses the mana-value alternative cost"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_actions_includes_rooftop_storm_free_cast_only_for_zombies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let rooftop_storm = CardDefinitionBuilder::new(CardId::from_raw(1450), "Rooftop Storm")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.",
            )
            .expect("Rooftop Storm text should parse");
        game.create_object_from_definition(&rooftop_storm, alice, Zone::Battlefield);

        let zombie = CardBuilder::new(CardId::from_raw(1451), "Zombie Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
            ]))
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let zombie_id = game.create_object_from_card(&zombie, alice, Zone::Hand);

        let non_zombie = CardBuilder::new(CardId::from_raw(1452), "Human Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
            ]))
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let non_zombie_id = game.create_object_from_card(&non_zombie, alice, Zone::Hand);

        let granted = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, zombie_id, Zone::Hand, alice);
        assert_eq!(
            granted.len(),
            1,
            "Rooftop Storm should grant one hand alternative cost to Zombies"
        );
        assert_eq!(
            granted[0]
                .method
                .mana_cost()
                .expect("Rooftop Storm grant should have a mana cost")
                .generic_mana_total(),
            0,
            "Rooftop Storm should turn Zombie creature spells into zero-mana alternative casts"
        );

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == zombie_id || *spell_id == non_zombie_id
            )),
            "without mana, neither creature should be normally castable"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Hand,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == zombie_id
            )),
            "Rooftop Storm should surface a free hand cast action for Zombie creature spells"
        );
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Hand,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == non_zombie_id
            )),
            "Rooftop Storm should not grant a free cast to non-Zombie creature spells"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_can_cast_spell_with_non_targeted_prevent_all_damage_without_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(13000), "Sivvi Cast Probe")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
                .parse_text(
                    "Prevent all damage that would be dealt this turn to creatures you control.",
                )
                .expect("prevent-all damage line should parse as a non-targeted effect");

        let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
        game.player_mut(alice)
            .expect("player should exist")
            .mana_pool
            .add(ManaSymbol::White, 1);

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "spell should be castable without creatures because effect is non-targeted"
        );
    }

    #[test]
    fn test_compute_legal_targets_respects_cant_target_player_restriction() {
        use crate::target::{ChooseSpec, PlayerFilter};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.effect_store
            .cant_effects
            .cant_target_players
            .insert(bob);
        let targets = crate::game_loop::compute_legal_targets(
            &game,
            &ChooseSpec::Player(PlayerFilter::Any),
            alice,
            None,
        );

        assert!(
            !targets.contains(&crate::Target::Player(bob)),
            "untargetable player should not appear in legal target set: {targets:?}"
        );
        assert!(
            targets.contains(&crate::Target::Player(alice)),
            "other legal players should remain targetable: {targets:?}"
        );
    }

    #[test]
    fn test_auto_pass_decision_maker() {
        use crate::decisions::context::PriorityContext;

        let game = setup_game();
        let mut dm = AutoPassDecisionMaker;

        let ctx = PriorityContext::new(PlayerId::from_index(0), vec![LegalAction::PassPriority]);

        let response = dm.decide_priority(&game, &ctx);
        assert!(matches!(response, LegalAction::PassPriority));
    }

    #[test]
    fn test_numeric_input_decision_maker() {
        use crate::decisions::context::PriorityContext;

        let game = setup_game();

        // Test priority decisions with numeric input
        let mut dm = NumericInputDecisionMaker::from_strs(&["0", "1", ""]);

        let legal_actions = vec![
            LegalAction::PassPriority,
            LegalAction::PlayLand {
                land_id: ObjectId::from_raw(1),
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());

        // "0" should select PassPriority
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::PassPriority
        ));

        // "1" should select PlayLand
        let ctx2 = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());
        assert!(matches!(
            dm.decide_priority(&game, &ctx2),
            LegalAction::PlayLand { .. }
        ));

        // "" (empty) should default to PassPriority
        let ctx3 = PriorityContext::new(PlayerId::from_index(0), legal_actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx3),
            LegalAction::PassPriority
        ));
    }

    #[test]
    fn test_numeric_input_priority_commander_shortcut_single() {
        use crate::alternative_cast::CastingMethod;
        use crate::decisions::context::PriorityContext;
        use crate::zone::Zone;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["c"]);

        let actions = vec![
            LegalAction::PassPriority,
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(100),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::CastSpell {
                from_zone: Zone::Command,
                ..
            }
        ));
    }

    #[test]
    fn test_numeric_input_priority_commander_shortcut_indexed() {
        use crate::alternative_cast::CastingMethod;
        use crate::decisions::context::PriorityContext;
        use crate::zone::Zone;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["c1"]);

        let actions = vec![
            LegalAction::PassPriority,
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(101),
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            },
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(102),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(103),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Command,
                ..
            } if spell_id == ObjectId::from_raw(103)
        ));
    }

    #[test]
    fn test_commander_tax_applies_to_recasts_from_command_zone() {
        use crate::ability::Ability;
        use crate::cost::TotalCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let commander = CardBuilder::new(CardId::from_raw(2000), "Test Commander")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Command);
        game.set_as_commander(commander_id, alice);

        for idx in 0..2 {
            let land = CardBuilder::new(
                CardId::from_raw(2100 + idx),
                &format!("Green Source {}", idx),
            )
            .card_types(vec![CardType::Land])
            .build();
            let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
            game.object_mut(land_id)
                .expect("green source should exist")
                .abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "initial cast should be affordable with two mana"
        );

        game.record_commander_cast_from_command_zone(commander_id);
        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            !can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "recast should require commander tax"
        );

        for idx in 0..2 {
            let land = CardBuilder::new(
                CardId::from_raw(2200 + idx),
                &format!("Extra Green Source {}", idx),
            )
            .card_types(vec![CardType::Land])
            .build();
            let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
            game.object_mut(land_id)
                .expect("extra green source should exist")
                .abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "four mana should pay the taxed commander cost"
        );
    }

    #[test]
    fn test_numeric_input_may_choice() {
        use crate::decisions::context::BooleanContext;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["y", "n", "", "1"]);

        let ctx = BooleanContext {
            player: PlayerId::from_index(0),
            source: Some(ObjectId::from_raw(1)),
            description: "Test?".to_string(),
            source_name: None,
            ui_hints: crate::decisions::context::DecisionUiHints::default(),
        };

        // "y" = true
        assert!(dm.decide_boolean(&game, &ctx));

        // "n" = false
        assert!(!dm.decide_boolean(&game, &ctx));

        // "" = false
        assert!(!dm.decide_boolean(&game, &ctx));

        // "1" = true
        assert!(dm.decide_boolean(&game, &ctx));
    }

    /// Tests that tapped creatures cannot activate mana abilities with tap costs.
    ///
    /// Scenario: Alice controls an untapped Llanowar Elves (which has "{T}: Add {G}").
    /// When untapped, she should be able to activate the mana ability. After tapping it,
    /// she should no longer be able to activate the ability.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_activated_ability_tap_cost_validation() {
        use crate::cards::definitions::llanowar_elves;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase (for priority)
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves on battlefield (has {T}: Add {G} - a mana ability)
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Remove summoning sickness so it can tap
        game.remove_summoning_sickness(creature_id);

        // Check legal actions - should include the mana ability
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should be able to activate untapped creature's tap mana ability"
        );

        // Now tap the creature (simulating it was already tapped for mana earlier)
        game.tap(creature_id);

        // Check legal actions again - should NOT include the mana ability
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate already-tapped creature's tap mana ability"
        );
    }

    #[test]
    fn test_activated_ability_mana_cost_validation() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create a creature with an activated ability that costs {1}{G}
        let creature = CardBuilder::new(CardId::from_raw(1), "Pump Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        // Add an activated ability: {1}{G}: +2/+2 until EOT
        let mana_cost =
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Green]]);
        let activated_ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                    2,
                    2,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        };
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(activated_ability);
        game.remove_summoning_sickness(creature_id);

        // Cost payment is validated during the activation flow, so the action
        // should still surface even before the player floats mana.
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should surface the activation even before mana is available"
        );

        // Add mana to pool
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        // Now should be able to activate
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should be able to activate with sufficient mana"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_tayam_wall_of_roots_activation_uses_mana_sequence_solver() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let tayam_id =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        let wall_id =
            game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

        if let Some(wall) = game.object_mut(wall_id) {
            wall.counters.insert(CounterType::MinusOneMinusOne, 2);
        }

        // Start with only 2 mana and 2 counters; activation should still be legal
        // because Wall of Roots can be activated during cost payment.
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 2);
        }

        let tayam_ability_index = game
            .object(tayam_id)
            .expect("Tayam should exist")
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .expect("Tayam should have an activated ability");

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == tayam_id && *ability_index == tayam_ability_index
            )),
            "Tayam activation should be legal when Wall of Roots can provide the 3rd mana and 3rd counter during payment"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_tayam_wall_of_roots_activation_blocked_when_wall_already_used() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let tayam_id =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        let wall_id =
            game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

        if let Some(wall) = game.object_mut(wall_id) {
            wall.counters.insert(CounterType::MinusOneMinusOne, 2);
        }

        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 2);
        }

        let wall_mana_ability_index = game
            .object(wall_id)
            .expect("Wall of Roots should exist")
            .abilities
            .iter()
            .position(|ability| ability.is_mana_ability())
            .expect("Wall of Roots should have a mana ability");
        game.record_ability_activation(wall_id, wall_mana_ability_index);

        let tayam_ability_index = game
            .object(tayam_id)
            .expect("Tayam should exist")
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .expect("Tayam should have an activated ability");

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == tayam_id && *ability_index == tayam_ability_index
            )),
            "Tayam activation should still surface even when the payment flow will reject it"
        );
    }

    #[test]
    fn test_activated_ability_cost_reduction_respects_minimum_one_mana() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Creature with two activated abilities: one costs {2}, one costs {1}.
        let creature = CardBuilder::new(CardId::from_raw(11), "Reducer Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);

        let cost_two = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]);
        let cost_one = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]);
        let activated = |cost: ManaCost| Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        };
        game.object_mut(creature_id)
            .expect("creature exists")
            .abilities
            .extend([activated(cost_two), activated(cost_one)]);

        // Training Grounds-style static ability.
        let reducer = CardBuilder::new(CardId::from_raw(12), "Training Grounds Effect")
            .card_types(vec![CardType::Enchantment])
            .build();
        let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
        game.object_mut(reducer_id)
            .expect("reducer exists")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::reduce_activated_ability_costs(
                    ObjectFilter::creature().you_control(),
                    2,
                    Some(1),
                ),
            ));

        let actions_without_mana = compute_legal_actions(&game, alice);
        assert!(
            actions_without_mana.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 0
                } if *source == creature_id
            )),
            "reduced activated abilities should still surface before mana is floated"
        );

        game.player_mut(alice)
            .expect("player exists")
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions_with_one = compute_legal_actions(&game, alice);
        assert!(
            actions_with_one.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 0
                } if *source == creature_id
            )),
            "with one mana, {{2}} ability should be reduced to {{1}}"
        );
        assert!(
            actions_with_one.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 1
                } if *source == creature_id
            )),
            "minimum-one-mana floor should keep {{1}} ability at {{1}}"
        );

        let reduced_one = calculate_effective_activation_total_cost(
            &game,
            alice,
            creature_id,
            &TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
        );
        assert_eq!(
            reduced_one
                .mana_cost()
                .expect("reduced cost keeps mana component")
                .generic_mana_total(),
            1,
            "minimum-one-mana floor should not reduce a {{1}} activation cost to zero"
        );
    }

    #[test]
    fn test_self_hand_activated_ability_cost_reduction_counts_matching_battlefield_objects() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let legend = CardBuilder::new(CardId::from_raw(21), "Legendary Scout")
            .supertypes(vec![crate::types::Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&legend, alice, Zone::Battlefield);
        game.create_object_from_card(&legend, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(22), "Hand Reducer")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source_id)
            .expect("source exists")
            .abilities
            .extend([
                Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: TotalCost::mana(ManaCost::from_pips(vec![vec![
                            ManaSymbol::Generic(3),
                        ]])),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::draw(1),
                        ]),
                        choices: vec![],
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: vec![Zone::Hand],
                },
                Ability::static_ability(StaticAbility::reduce_activated_ability_costs_for_each(
                    ObjectFilter::source(),
                    1,
                    ObjectFilter::creature()
                        .you_control()
                        .with_supertype(crate::types::Supertype::Legendary),
                    Some(1),
                ))
                .in_zones(vec![Zone::Battlefield, Zone::Hand]),
            ]);

        let reduced = calculate_effective_activation_mana_cost(
            &game,
            alice,
            source_id,
            &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
        );
        assert_eq!(
            reduced.generic_mana_total(),
            1,
            "two matching legendary creatures should reduce a hand-zone activation from {{3}} to {{1}}"
        );
    }

    #[test]
    fn test_activated_ability_cost_reduction_counts_distinct_basic_land_types() {
        use crate::ability::Ability;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(31), "Domain Codex")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source exists")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::reduce_activated_ability_costs_for_each_basic_land_type(
                    ObjectFilter::source(),
                    1,
                    ObjectFilter::land().you_control(),
                    Some(1),
                ),
            ));

        for (id, name, subtype) in [
            (32, "Plains", Subtype::Plains),
            (33, "Snowfield", Subtype::Plains),
            (34, "Island", Subtype::Island),
        ] {
            let land = CardBuilder::new(CardId::from_raw(id), name)
                .card_types(vec![CardType::Land])
                .subtypes(vec![subtype])
                .build();
            game.create_object_from_card(&land, alice, Zone::Battlefield);
        }

        let reduced = calculate_effective_activation_mana_cost(
            &game,
            alice,
            source_id,
            &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]),
        );
        assert_eq!(
            reduced.generic_mana_total(),
            3,
            "three lands with two basic land types should reduce {{5}} by two"
        );
    }

    /// Tests that summoning sick creatures cannot activate mana abilities with tap costs.
    ///
    /// Scenario: Alice casts Llanowar Elves. On the same turn, the creature has
    /// summoning sickness, so she should not be able to activate its "{T}: Add {G}"
    /// mana ability.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_activated_ability_summoning_sickness_blocks_tap() {
        use crate::cards::definitions::llanowar_elves;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves on battlefield with summoning sickness
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Creature just entered battlefield, so it has summoning sickness
        game.set_summoning_sick(creature_id);

        // Should NOT be able to activate tap mana ability due to summoning sickness
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Summoning sick creature should not be able to use tap mana abilities"
        );
    }

    /// Tests that creatures with haste can use tap mana abilities despite summoning sickness.
    ///
    /// Scenario: Alice has given her Llanowar Elves haste (e.g., via an effect like
    /// Swiftfoot Boots). Even though the creature just entered the battlefield and
    /// has summoning sickness, haste allows it to activate its "{T}: Add {G}" mana ability.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_activated_ability_haste_bypasses_summoning_sickness() {
        use crate::ability::Ability;
        use crate::cards::definitions::llanowar_elves;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves with summoning sickness but also with haste
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Add haste (e.g., from equipment or an enchantment)
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::haste()));

        // Creature just entered battlefield, so it has summoning sickness
        game.set_summoning_sick(creature_id);

        // Should be able to activate tap mana ability despite summoning sickness (has haste)
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Creature with haste should be able to use tap mana abilities despite summoning sickness"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_turn_face_up_for_morph() {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        let creature = CardBuilder::new(CardId::from_raw(101), "Morph Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::morph(
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Green,
                ]])),
            )));
        game.set_face_down(creature_id);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Green, 1);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(
                |a| matches!(a, LegalAction::TurnFaceUp { creature_id: id, .. } if *id == creature_id)
            ),
            "face-down creature with payable morph cost should have TurnFaceUp legal action"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_face_down_cast_for_morph_when_normal_cast_is_too_expensive()
     {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(102), "Costly Morph Bear")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![
                vec![crate::mana::ManaSymbol::Generic(5)],
                vec![crate::mana::ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::morph(
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Green,
                ]])),
            )));
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 3);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::FaceDown,
                } if *spell_id == creature_id
            )),
            "morph card should be castable face down when {{3}} is payable"
        );
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == creature_id
            )),
            "normal cast should stay unavailable when the printed mana cost is too expensive"
        );
    }

    #[test]
    fn test_activated_ability_sorcery_speed_timing() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::game_state::Step;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with sorcery-speed activated ability
        let creature = CardBuilder::new(CardId::from_raw(1), "Sorcery Speed Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        // Add sorcery-speed activated ability (no cost, just free)
        let activated_ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        };
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(activated_ability);
        game.remove_summoning_sickness(creature_id);

        // Main phase, empty stack - should be able to activate
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should be able to activate sorcery-speed ability during main phase with empty stack"
        );

        // Combat phase - should NOT be able to activate
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate sorcery-speed ability during combat"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_hand_activated_ability() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let card = CardBuilder::new(CardId::from_raw(777), "Hand Ability Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source_id)
            .expect("source card should exist")
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::free(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::gain_life(1),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Hand],
            });

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(
                |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == source_id)
            ),
            "hand-zone activated ability should be discoverable as a legal action"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_legal_actions_excludes_hand_only_ability_from_battlefield() {
        use crate::cards::CardDefinitionBuilder;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let def = CardDefinitionBuilder::new(CardId::from_raw(779), "Boseiju Regression Probe")
            .card_types(vec![CardType::Land])
            .mana_cost(ManaCost::new())
            .parse_text(
                "{T}: Add {G}.\nChannel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls.\nThis ability costs {1} less to activate for each legendary creature you control.",
            )
            .expect("channel land probe should parse");

        let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateManaAbility { source, .. } if *source == source_id
            )),
            "battlefield Boseiju should still expose its tap-for-mana ability"
        );
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "battlefield Boseiju should not expose its hand-only channel ability"
        );
    }

    #[test]
    fn test_tap_only_activation_skips_empty_next_cost_prompt() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;
        use crate::game_loop::{
            PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
        };
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(780), "Tap Probe")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("tap probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::tap()],
                vec![Effect::gain_life(1)],
            ));

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = AutoPassDecisionMaker;

        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
                source: source_id,
                ability_index: 0,
            }),
            &mut dm,
        )
        .expect("tap-only activation should resolve its cost flow");

        assert!(
            state.pending_activation.is_none(),
            "tap-only activation should not get stuck in a pending cost prompt"
        );
        assert!(
            game.is_tapped(source_id),
            "tap-only activation should pay the tap cost immediately"
        );
        assert_eq!(
            game.stack.len(),
            1,
            "tap-only activation should place the ability on the stack"
        );
        assert!(
            matches!(
                progress,
                crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
            ),
            "after a tap-only activation resolves its cost, priority should continue normally"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_tapped_non_mana_tap_ability() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(781), "Tapped Ability Probe")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::tap()],
                vec![Effect::gain_life(1)],
            ));
        game.tap(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "tapped permanents should not expose non-mana tap abilities as legal actions"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_summoning_sick_non_mana_untap_ability() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(782), "Untap Ability Probe")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::untap()],
                vec![Effect::gain_life(1)],
            ));
        game.tap(source_id);
        game.set_summoning_sick(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "summoning-sick creatures should not expose non-mana untap abilities as legal actions"
        );

        game.remove_summoning_sickness(source_id);
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "once summoning sickness is removed, the untap ability should become legal"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_foretell_special_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(778), "Foretell Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .foretell(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Foretell {
                    card_id: found
                }) if *found == card_id
            )),
            "expected foretell special action in legal actions, got {actions:?}"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_suspend_special_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(779), "Suspend Probe")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "expected suspend special action in legal actions, got {actions:?}"
        );
    }

    #[test]
    fn test_suspend_special_action_respects_cant_cast_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(7791),
            "Suspend Restriction Probe",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        game.effect_store.cant_effects.add_cant_cast_filter(
            alice,
            crate::target::ObjectFilter::default().with_type(CardType::Creature),
        );

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "suspend should not be offered when a cast prohibition would stop starting the cast, got {actions:?}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_suspend_only_card_does_not_offer_normal_cast_from_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Lotus Bloom")
            .parse_text(
                "Type: Artifact\n\
                 Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
                 {T}, Sacrifice this artifact: Add three mana of any one color.",
            )
            .expect("Lotus Bloom text should parse");
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == card_id
            )),
            "suspend-only card should not offer a normal cast action, got {actions:?}"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "suspend-only card should still offer suspend, got {actions:?}"
        );
    }

    #[test]
    fn test_suspend_special_action_exiles_with_time_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(781), "Suspend Runtime")
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![Effect::gain_life(1)])
                .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
                .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Suspend { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("suspend special action should resolve");

        let exiled_id = *game.exile.first().expect("card should be exiled");
        let exiled = game.object(exiled_id).expect("exiled card should exist");
        assert_eq!(exiled.zone, Zone::Exile);
        assert_eq!(
            game.counter_count(exiled_id, crate::object::CounterType::Time),
            2
        );
    }

    #[test]
    fn test_plot_special_action_enables_cast_on_later_turn_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 3);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(780), "Plot Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .plot(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Plot { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("plot special action should resolve");

        let exiled_id = *game.exile.first().expect("card should be in exile");
        let same_turn_actions = compute_legal_actions(&game, alice);
        assert!(
            !same_turn_actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == exiled_id
            )),
            "plotted card should not be castable the same turn it was plotted"
        );

        game.next_turn();
        game.next_turn();
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let later_actions = compute_legal_actions(&game, alice);
        assert!(
            later_actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == exiled_id
            )),
            "plotted card should be castable from exile on a later turn"
        );
    }

    #[test]
    fn test_spectacle_condition_controls_alternative_cast_legality() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);

        let def =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Spectacle Probe")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(2)],
                    vec![ManaSymbol::Red],
                ]))
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![Effect::gain_life(1)])
                .spectacle(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
                .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);
        let card = game.object(card_id).expect("spectacle card should exist");
        assert!(
            !can_cast_with_alternative_from_hand(
                &game,
                alice,
                card,
                card_id,
                &card.alternative_casts[0]
            ),
            "spectacle alternative should not be available before an opponent loses life"
        );

        stage_life_loss_for_test(&mut game, bob, 1);
        let card = game
            .object(card_id)
            .expect("spectacle card should still exist");
        assert!(
            can_cast_with_alternative_from_hand(
                &game,
                alice,
                card,
                card_id,
                &card.alternative_casts[0]
            ),
            "spectacle alternative should become available once an opponent has lost life"
        );
    }

    #[test]
    fn test_foretell_special_action_enables_cast_from_exile() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 4);

        let def = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(779),
            "Foretell Runtime Probe",
        )
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .foretell(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Foretell { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("foretell special action should succeed");

        let foretold_id = *game.exile.last().expect("card should be in exile");
        assert!(game.is_face_down(foretold_id));
        assert!(game.is_foretold(foretold_id));

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == foretold_id
            )),
            "expected foretold card to be castable from exile, got {actions:?}"
        );
    }

    /// Tests that compute_potential_mana correctly calculates mana from untapped sources.
    ///
    /// Scenario: Player has empty mana pool but 4 untapped Mountains on battlefield.
    /// compute_potential_mana should return a pool with 4 red mana.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_compute_potential_mana_with_untapped_lands() {
        use crate::cards::definitions::basic_mountain;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Verify mana pool is empty
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            0,
            "Mana pool should start empty"
        );

        // Create 4 Mountains on battlefield
        let mountain_def = basic_mountain();
        for _ in 0..4 {
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
        }

        // compute_potential_mana should include mana from untapped lands
        let potential = compute_potential_mana(&game, alice);
        assert_eq!(
            potential.red, 4,
            "Should have 4 potential red mana from Mountains"
        );
        assert_eq!(potential.total(), 4, "Total potential mana should be 4");
    }

    /// Tests that max_x_for_cost works correctly with potential mana.
    ///
    /// Scenario: Player has empty mana pool but 4 untapped Mountains.
    /// For a Fireball ({X}{R}), max X should be 3 (4 total mana - 1 for {R} = 3 for X).
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_max_x_with_potential_mana() {
        use crate::cards::definitions::basic_mountain;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Verify mana pool is empty
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            0,
            "Mana pool should start empty"
        );

        // Create 4 Mountains on battlefield
        let mountain_def = basic_mountain();
        for _ in 0..4 {
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
        }

        // Fireball cost: {X}{R}
        let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

        // Using just the mana pool (which is empty), max_x would be 0
        let max_x_from_pool = game
            .player(alice)
            .unwrap()
            .mana_pool
            .max_x_for_cost(&fireball_cost);
        assert_eq!(max_x_from_pool, 0, "max_x from empty pool should be 0");

        // Using potential mana (including untapped lands), max_x should be 3
        let potential = compute_potential_mana(&game, alice);
        let max_x_from_potential = potential.max_x_for_cost(&fireball_cost);
        assert_eq!(
            max_x_from_potential, 3,
            "max_x from potential mana should be 3 (4 mana - 1 for R = 3 for X)"
        );
    }

    /// Tests that potential mana includes mana dorks (creatures with mana abilities).
    ///
    /// Scenario: Player has 1 Mountain and 1 Llanowar Elves (untapped, no summoning sickness).
    /// For Fireball ({X}{R}), max X should be 1 (2 total mana - 1 for {R} = 1 for X).
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_max_x_with_mana_dork() {
        use crate::cards::definitions::{basic_mountain, llanowar_elves};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Mountain and Llanowar Elves
        let mountain_def = basic_mountain();
        game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);

        let elves_def = llanowar_elves();
        let elves_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(elves_id);

        // Fireball cost: {X}{R}
        let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

        // Potential mana: 1R from Mountain + 1G from Elves = 2 total
        let potential = compute_potential_mana(&game, alice);
        assert_eq!(potential.red, 1, "Should have 1 potential red mana");
        assert_eq!(potential.green, 1, "Should have 1 potential green mana");
        assert_eq!(potential.total(), 2, "Total potential mana should be 2");

        // max_x should be 1: pay {R} with Mountain, {X}=1 with Elves' green mana
        let max_x = potential.max_x_for_cost(&fireball_cost);
        assert_eq!(max_x, 1, "max_x should be 1 (2 total - 1 for R = 1 for X)");
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_graveyard_play_from_actions_include_variable_mana_sources() {
        use crate::cards::definitions::lightning_bolt;
        use crate::cards::tokens::treasure_token_definition;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Treasure's mana ability is effect-backed ("any color"), so this specifically
        // verifies variable mana producers are considered in castability checks.
        let treasure = treasure_token_definition();
        game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

        let bolt = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.effect_store
            .grant_registry
            .grant_to_filter_until_end_of_turn(
                ObjectFilter::nonland(),
                Zone::Graveyard,
                alice,
                Grantable::play_from(),
                source_id,
                game.turn.turn_number,
            );

        let actions = compute_legal_actions(&game, alice);
        let can_cast_from_graveyard = actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Graveyard,
                        ..
                    },
                    ..
                } if *spell_id == bolt_id
            )
        });

        assert!(
            can_cast_from_graveyard,
            "variable mana sources should allow castability inference for play-from-graveyard actions"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_counter_unless_pays_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::{basic_island, mana_tithe};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice the mana to cast Mana Tithe.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // Put Mana Tithe in hand and leave stack empty.
        let mana_tithe_def = mana_tithe();
        let mana_tithe_id = game.create_object_from_definition(&mana_tithe_def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == mana_tithe_id
            )
        });

        assert!(
            !can_cast,
            "counter-unless-pays spells must not be castable without a legal spell target on stack"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_conditional_counter_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::basic_island;
        use crate::effect::Condition;
        use crate::game_state::StackEntry;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Give Alice the mana to cast the conditional counterspell.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // Corrupted Resolve-shaped payload:
        // "Counter target spell if its controller is poisoned."
        let card = CardBuilder::new(CardId::from_raw(91), "Corrupted Resolve Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::conditional(
                Condition::TargetSpellControllerIsPoisoned,
                vec![Effect::counter(ChooseSpec::target_spell())],
                vec![],
            ),
        ]));

        // With no spell on stack, the counterspell must not be castable.
        let actions_without_stack = compute_legal_actions(&game, alice);
        let can_cast_without_stack = actions_without_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            !can_cast_without_stack,
            "conditional counterspell should not be castable without a legal spell target on stack"
        );

        // Add a dummy spell to the stack and verify the cast action appears.
        let dummy_spell = CardBuilder::new(CardId::from_raw(92), "Stack Dummy")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
        game.push_to_stack(StackEntry::new(dummy_id, bob));

        let actions_with_stack = compute_legal_actions(&game, alice);
        let can_cast_with_stack = actions_with_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            can_cast_with_stack,
            "conditional counterspell should be castable once a legal spell target exists on stack"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_if_effect_counter_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::basic_island;
        use crate::game_state::StackEntry;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Give Alice the mana to cast the spell.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // "If you do, counter target spell." shape:
        // represented as IfEffect branching on a prior effect result.
        let card = CardBuilder::new(CardId::from_raw(93), "If Counter Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::if_then(
                crate::effect::EffectId(0),
                crate::effect::EffectPredicate::Happened,
                vec![Effect::counter(ChooseSpec::target_spell())],
            ),
        ]));

        // With no spell on stack, the spell must not be castable.
        let actions_without_stack = compute_legal_actions(&game, alice);
        let can_cast_without_stack = actions_without_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            !can_cast_without_stack,
            "if-effect counterspell should not be castable without a legal spell target on stack"
        );

        // Add a legal stack spell; cast action should appear.
        let dummy_spell = CardBuilder::new(CardId::from_raw(94), "If Stack Dummy")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
        game.push_to_stack(StackEntry::new(dummy_id, bob));

        let actions_with_stack = compute_legal_actions(&game, alice);
        let can_cast_with_stack = actions_with_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            can_cast_with_stack,
            "if-effect counterspell should be castable once a legal spell target exists on stack"
        );
    }
}
