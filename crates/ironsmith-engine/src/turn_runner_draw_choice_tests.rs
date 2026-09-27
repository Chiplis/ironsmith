use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::CardDefinition;
use crate::effect::Effect;
use crate::events::cards::matchers::WouldDrawCardMatcher;
use crate::events::other::CardsDrawnEvent;
use crate::game_state::MeldComponentState;
use crate::ids::{CardId, StableId};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::triggers::Trigger;
use crate::types::CardType;
use crate::zone::Zone;

fn creature(name: &str) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
    )
}

#[test]
fn modified_draw_batch_waits_for_commander_in_either_library_position() {
    for commander_on_top in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.turn_number = 2;
        let watcher = game.create_object_from_definition(
            &creature("Draw doubler and watcher"),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(watcher)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::you_draw_cards(),
                vec![Effect::gain_life(1)],
            ));
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                watcher,
                alice,
                WouldDrawCardMatcher::you(),
                ReplacementAction::Double,
            ),
        );
        let ordinary = creature("Ordinary drawn card");
        let commander = creature("Draw batch commander");
        let cards = if commander_on_top {
            [&ordinary, &commander]
        } else {
            [&commander, &ordinary]
        };
        let mut commander_id = None;
        for card in cards {
            let id = game.create_object_from_definition(card, alice, Zone::Library);
            if card.card.name == commander.card.name {
                commander_id = Some(id);
            }
        }
        game.set_as_commander(commander_id.unwrap(), alice);
        let library_before = game.player(alice).unwrap().library.to_vec();
        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut tq = TriggerQueue::new();
        for _ in 0..2 {
            let action = runner.advance(&mut game, &mut tq).unwrap();
            let TurnAction::Decision(DecisionContext::Boolean(context)) = action else {
                panic!("drawn commander must prompt even in a modified draw batch: {action:?}");
            };
            assert_eq!(context.player, alice);
            assert!(context.description.contains("command zone"));
            assert_eq!(game.player(alice).unwrap().library.to_vec(), library_before);
            assert!(game.player(alice).unwrap().hand.is_empty());
            assert!(game.objects_in_zone(Zone::Command).is_empty());
            assert!(tq.is_empty());
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
        }
        runner.respond_boolean(true);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert!(game.player(alice).unwrap().library.is_empty());
        let hand = &game.player(alice).unwrap().hand;
        assert_eq!(hand.len(), 1);
        assert_eq!(
            game.object(hand[0]).unwrap().name.as_str(),
            "Ordinary drawn card"
        );
        let command = game.objects_in_zone(Zone::Command);
        assert_eq!(command.len(), 1);
        assert_eq!(
            game.object(command[0]).unwrap().name.as_str(),
            "Draw batch commander"
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
        assert_eq!(
            tq.entries.len(),
            1,
            "the completed batch emits CardsDrawn once"
        );
        let event = tq.entries[0]
            .triggering_event
            .downcast::<CardsDrawnEvent>()
            .unwrap();
        assert_eq!(event.amount(), 1);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
        assert_eq!(
            tq.entries.len(),
            1,
            "continuing must not repeat the draw event"
        );
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }
}

#[test]
fn lethal_meld_sba_waits_for_graveyard_order_without_replacement_effects() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let melded =
        game.create_object_from_definition(&creature("Melded creature"), alice, Zone::Battlefield);
    let mut components = Vec::new();
    for name in ["First meld component", "Second meld component"] {
        game.register_linked_face_definition(&creature(name));
        components.push(MeldComponentState {
            stable_id: StableId::from_object_id(game.new_object_id()),
            owner: alice,
            name: name.into(),
        });
    }
    game.set_melded_permanent(melded, components);
    game.mark_damage(melded, 2);
    game.refresh_continuous_state();
    assert!(game.effect_store.replacement_effects.effects().is_empty());
    let mut runner = TurnRunner::new();
    let mut tq = TriggerQueue::new();
    let mut answer = None;
    for _ in 0..2 {
        let progress = runner
            .apply_sbas_until_commander_choice(&mut game, &mut tq)
            .unwrap();
        let RunnerProgress::NeedsDecision(DecisionContext::Order(context)) = progress else {
            panic!("split meld cards require a graveyard ordering decision");
        };
        assert_eq!(context.player, alice);
        assert!(context.description.contains("graveyard"));
        assert_eq!(context.items.len(), 2);
        let reverse = context
            .items
            .iter()
            .rev()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        assert_eq!(
            answer.get_or_insert(reverse.clone()),
            &reverse,
            "trial-created card IDs must remain stable on replay"
        );
        assert!(game.battlefield.contains(&melded));
        assert_eq!(game.damage_on(melded), 2);
        assert!(game.player(alice).unwrap().graveyard.is_empty());
    }
    let answer = answer.unwrap();
    runner.respond_order(answer.clone());
    assert!(matches!(
        runner
            .apply_sbas_until_commander_choice(&mut game, &mut tq)
            .unwrap(),
        RunnerProgress::Complete(())
    ));
    assert!(!game.battlefield.contains(&melded));
    assert_eq!(game.player(alice).unwrap().graveyard.to_vec(), answer);
    assert_eq!(
        game.object(answer[0]).unwrap().name.as_str(),
        "Second meld component"
    );
    assert_eq!(
        game.object(answer[1]).unwrap().name.as_str(),
        "First meld component"
    );
}
