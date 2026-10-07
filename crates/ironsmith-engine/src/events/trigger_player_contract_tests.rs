use super::*;
use crate::ability::Ability;
use crate::cards::builders::CardDefinitionBuilder;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{ExecutionContext, helpers::resolve_player_filter};
use crate::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use crate::target::PlayerFilter;
use crate::triggers::{Trigger, TriggerEvent, TriggerQueue, check_triggers};
use crate::{CardId, CardType, Effect, GameState};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const CARA: PlayerId = PlayerId(2);

fn coin_flip(player: PlayerId, won: bool) -> CoinFlippedEvent {
    CoinFlippedEvent {
        turn_ordinal: 0,
        instruction_ordinal: 0,
        player,
        source: ObjectId::from_raw(99),
        face: ironsmith_core::CoinFace::Heads,
        call: Some(if won {
            ironsmith_core::CoinFace::Heads
        } else {
            ironsmith_core::CoinFace::Tails
        }),
        winner: won.then_some(player),
        loser: (!won).then_some(player),
    }
}

#[test]
fn player_event_contract_binds_the_event_participant_in_resolution_context() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = CARA;
    let object = ObjectId::from_raw(99);
    let events: Vec<Box<dyn GameEventType>> = vec![
        Box::new(LifeLossEvent::from_effect(BOB, 2)),
        Box::new(LifeLossEvent::new(BOB, 2, true)),
        Box::new(LifeGainEvent::new(BOB, 2)),
        Box::new(coin_flip(BOB, true)),
        Box::new(coin_flip(BOB, false)),
        Box::new(SpellCounteredEvent::new(object, BOB, None)),
        Box::new(SpellCastEvent::new(object, BOB, Zone::Hand)),
        Box::new(CardsDrawnEvent::single(BOB, object, false)),
        Box::new(CardDiscardedEvent::new(BOB, object)),
        Box::new(ShuffleLibraryEvent::new(BOB, EventCause::effect())),
        Box::new(BeginningOfUpkeepEvent::new(BOB)),
        Box::new(BeginningOfEndStepEvent::new(BOB)),
        Box::new(other::DieRolledEvent::new(BOB, object, 4, 6)),
    ];

    for event in events {
        let event = TriggerEvent::from_boxed(event, Default::default());
        let kind = event.kind();
        let context = ExecutionContext::new_default(object, ALICE).with_triggering_event(event);
        assert_eq!(
            resolve_player_filter(&game, &PlayerFilter::IteratedPlayer, &context),
            Ok(BOB),
            "{kind:?} must bind its participant independently of the ability controller and active player"
        );
    }
}

#[test]
fn coin_flip_and_countered_spell_triggers_retain_the_player_through_the_stack() {
    for (trigger, event) in [
        (
            Trigger::player_coin_flip_result(PlayerFilter::Any, true),
            TriggerEvent::new(coin_flip(BOB, true), Default::default()),
        ),
        (
            Trigger::player_coin_flip_result(PlayerFilter::Any, false),
            TriggerEvent::new(coin_flip(BOB, false), Default::default()),
        ),
        (
            Trigger::spell_countered(None, PlayerFilter::Any),
            TriggerEvent::new(
                SpellCounteredEvent::new(ObjectId::from_raw(99), BOB, None),
                Default::default(),
            ),
        ),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        game.turn.active_player = CARA;
        let definition = CardDefinitionBuilder::new(CardId::new(), "Player event observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                trigger,
                vec![Effect::lose_life_player(2, PlayerFilter::IteratedPlayer)],
            ))
            .build();
        let source = game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
        let entries = check_triggers(&game, &event);
        assert_eq!(
            entries.len(),
            1,
            "{:?} must match the observer",
            event.kind()
        );
        assert_eq!(entries[0].source, source);
        let mut queue = TriggerQueue::new();
        for entry in entries {
            queue.add(entry);
        }
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .expect("trigger should be put on the stack");
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .expect("that-player reference should resolve from the triggering event");
        assert_eq!(game.life_total(ALICE), 20);
        assert_eq!(game.life_total(BOB), 18);
        assert_eq!(game.life_total(CARA), 20);
    }
}
