use super::*;

const TEXT: &str = "Whenever this creature attacks, if it doesn't have first strike, put a first strike counter on it.\nWhenever this creature attacks, if it has first strike, it gains double strike until end of turn.";

#[test]
fn attack_keyword_progression_checks_first_strike_at_trigger_and_resolution() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Momentum Rumbler")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .parse_text(TEXT)
            .unwrap();
    for starts_with_first_strike in [false, true] {
        for change_before_resolution in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if starts_with_first_strike {
                game.add_counters(source, CounterType::FirstStrike, 1);
            }
            game.refresh_continuous_state();
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::CreatureAttackedEvent::new(
                    source,
                    crate::events::AttackEventTarget::Player(bob),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(
                triggers.len(),
                1,
                "only the condition true at attack time triggers"
            );
            if change_before_resolution {
                if starts_with_first_strike {
                    game.remove_counters(source, CounterType::FirstStrike, 1, None, None);
                } else {
                    game.add_counters(source, CounterType::FirstStrike, 1);
                }
                game.refresh_continuous_state();
            }
            let trigger = &triggers[0];
            let condition = trigger
                .ability
                .intervening_if
                .as_ref()
                .expect("intervening if condition");
            let resolves = crate::triggers::verify_intervening_if(
                &game,
                condition,
                alice,
                &event,
                source,
                Some(trigger.trigger_identity),
                None,
            );
            assert_eq!(resolves, !change_before_resolution);
            if resolves {
                let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                    .with_triggering_event(event);
                for segment in &trigger.ability.effects.segments {
                    for effect in &segment.default_effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
            }
            game.refresh_continuous_state();
            assert_eq!(
                game.counter_count(source, CounterType::FirstStrike),
                u32::from(!(starts_with_first_strike && change_before_resolution))
            );
            assert_eq!(
                game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::DoubleStrike
                ),
                starts_with_first_strike && !change_before_resolution
            );
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            assert!(!game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::DoubleStrike
            ));
        }
    }
}

#[test]
fn attack_keyword_progression_renders_conditions_and_shared_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Momentum Rumbler")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn attack_keyword_progression_supports_other_keyword_pairs() {
    let text = TEXT
        .replace("first strike", "flying")
        .replace("double strike", "haste");
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Keyword Progression Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(&text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
