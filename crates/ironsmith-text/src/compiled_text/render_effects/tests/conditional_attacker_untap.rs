use super::*;
use crate::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
use crate::ids::{CardId, PlayerId};
const TEXT: &str = "Prevent all combat damage that would be dealt this turn.\nFateful hour — If you have 5 or less life, tap all attacking creatures. Those creatures don't untap during their controller's next untap step.";

#[test]
fn conditional_tapped_attackers_keep_the_next_untap_restriction() {
    for life in [5, 6] {
        for initially_tapped in [false, true] {
            let definition = CardDefinitionBuilder::new(CardId::new(), "Clinging Mists")
                .card_types(vec![CardType::Instant])
                .parse_text(TEXT)
                .unwrap();
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], life);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let creature = CardDefinitionBuilder::new(CardId::new(), "Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            let attacker = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            let other = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            if initially_tapped {
                game.tap(attacker);
            }
            game.combat = Some(CombatState {
                attackers: vec![AttackerInfo {
                    creature: attacker,
                    target: AttackTarget::Player(alice),
                }],
                ..CombatState::default()
            });
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            for effect in definition.spell_effect.as_ref().unwrap() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.is_tapped(attacker), initially_tapped || life <= 5);
            game.combat = None;
            assert!(
                game.can_untap(attacker),
                "ordinary untap effects remain legal outside the untap step"
            );
            game.turn.active_player = bob;
            game.turn.phase = crate::game_state::Phase::Beginning;
            game.turn.step = Some(crate::game_state::Step::Untap);
            game.update_cant_effects();
            assert_eq!(
                game.can_untap_during_step(attacker, bob),
                life > 5,
                "life={life}, initially_tapped={initially_tapped}"
            );
            assert!(game.can_untap_during_step(other, bob));
            crate::turn::execute_untap_step(&mut game);
            assert_eq!(game.is_tapped(attacker), life <= 5);
            game.next_turn();
            crate::turn::execute_untap_step(&mut game);
            game.next_turn();
            crate::turn::execute_untap_step(&mut game);
            assert!(
                !game.is_tapped(attacker),
                "the second controller untap step is unrestricted"
            );
        }
    }
}

#[test]
fn conditional_attacker_untap_text() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Clinging Mists")
        .card_types(vec![CardType::Instant])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.replace("Fateful hour — ", "")
    );
}
