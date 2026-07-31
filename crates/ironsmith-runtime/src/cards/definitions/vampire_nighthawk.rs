//! Vampire Nighthawk card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;

/// Vampire Nighthawk - {1}{B}{B}
/// Creature — Vampire Shaman (2/3)
/// Flying, deathtouch, lifelink
pub fn vampire_nighthawk() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vampire Nighthawk")
        .parse_text(
            "Mana cost: {1}{B}{B}\n\
             Type: Creature — Vampire Shaman\n\
             Power/Toughness: 2/3\n\
             Flying, deathtouch, lifelink",
        )
        .expect("Vampire Nighthawk text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::cards::definitions::{giant_spider, grizzly_bears};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::game_loop::{check_and_apply_sbas, execute_combat_damage_step};
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};
    use crate::triggers::TriggerQueue;
    use crate::zone::Zone;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_vampire_nighthawk() {
        let def = vampire_nighthawk();
        assert_eq!(def.name(), "Vampire Nighthawk");
        assert_eq!(
            def.abilities
                .iter()
                .filter(|ability| {
                    !matches!(
                        &ability.kind,
                        AbilityKind::Static(static_ability)
                            if static_ability.id()
                                == StaticAbilityId::SourceLineKeywordGroup
                    )
                })
                .count(),
            3
        );
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(2, 3)));

        for expected in [
            StaticAbilityId::Flying,
            StaticAbilityId::Deathtouch,
            StaticAbilityId::Lifelink,
        ] {
            assert!(
                def.abilities.iter().any(|ability| {
                    matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.id() == expected)
                }),
                "Vampire Nighthawk should have {expected:?}"
            );
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn vampire_nighthawk_keyword_text_matches_oracle_lines() {
        let def = vampire_nighthawk();
        let compiled = crate::compiled_text::unprocessed_compiled_lines(&def);
        let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
            crate::semantic_compare::compare_card_semantics_scored(
                "Vampire Nighthawk",
                "Flying\nDeathtouch\nLifelink",
                &compiled,
                crate::semantic_compare::report_embedding_config(),
            );

        assert_eq!(similarity, 1.0);
        assert_eq!(delta, 0);
        assert!(
            !mismatch,
            "keyword-only rendered text should compare cleanly against oracle lines: {compiled:?}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn vampire_nighthawk_keywords_matter_in_combat() {
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let nighthawk_id =
            game.create_object_from_definition(&vampire_nighthawk(), alice, Zone::Battlefield);
        let ground_blocker_id =
            game.create_object_from_definition(&grizzly_bears(), bob, Zone::Battlefield);
        let reach_blocker_id =
            game.create_object_from_definition(&giant_spider(), bob, Zone::Battlefield);

        assert!(
            !crate::rules::combat::can_block(
                game.object(nighthawk_id)
                    .expect("Vampire Nighthawk should exist"),
                game.object(ground_blocker_id)
                    .expect("ground blocker should exist"),
                &game,
            ),
            "a ground creature should not be able to block Vampire Nighthawk"
        );
        assert!(
            crate::rules::combat::can_block(
                game.object(nighthawk_id)
                    .expect("Vampire Nighthawk should exist"),
                game.object(reach_blocker_id)
                    .expect("reach blocker should exist"),
                &game,
            ),
            "a reach creature should be able to block Vampire Nighthawk"
        );

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: nighthawk_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(nighthawk_id, vec![reach_blocker_id]);
        combat
            .damage_assignment_order
            .insert(nighthawk_id, vec![reach_blocker_id]);

        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert!(
            events.iter().any(|event| {
                event.source == nighthawk_id && event.amount == 2 && event.result.life_gained == 2
            }),
            "Vampire Nighthawk should assign its full power to its only blocker and gain that much life"
        );
        assert_eq!(
            game.player(alice).expect("Alice should exist").life,
            22,
            "lifelink should gain Alice life equal to the damage dealt"
        );

        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue)
            .expect("state-based actions should apply after combat damage");

        assert!(
            game.player(bob)
                .expect("Bob should exist")
                .graveyard
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|object| object.name == "Giant Spider"),
            "deathtouch damage should destroy the reach blocker"
        );
        assert!(
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|object| object.name == "Vampire Nighthawk"),
            "Vampire Nighthawk should survive 1 damage from the blocker"
        );
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Vampire Nighthawk (creature with flying, deathtouch, lifelink).
    ///
    /// Vampire Nighthawk: {1}{B}{B} creature 2/3
    /// Flying, deathtouch, lifelink
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_vampire_nighthawk_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Vampire Nighthawk
                "0", // Tap first Swamp
                "0", // Tap second Swamp
                "0", // Tap third Swamp (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Vampire Nighthawk"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"]),
        );

        // Vampire Nighthawk should be on the battlefield
        assert!(
            game.battlefield_has("Vampire Nighthawk"),
            "Vampire Nighthawk should be on battlefield after casting"
        );
    }
}
