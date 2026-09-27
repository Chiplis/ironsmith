use super::*;
use crate::ability::Ability;
use crate::card::CardBuilder;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[test]
fn saga_lore_replacements_wait_for_every_saga_before_queuing_chapters() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let mut sagas = Vec::new();
    for name in ["First Saga", "Second Saga"] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .build();
        let saga = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(saga)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                crate::triggers::Trigger::saga_chapter(vec![1, 2, 3, 4]),
                vec![Effect::gain_life(1)],
            ));
        sagas.push(saga);
    }
    let mut replacements = Vec::new();
    for (name, ability) in [
        (
            "Extra lore",
            StaticAbility::add_counters_placement_replacement(
                ObjectFilter::default(),
                Some(CounterType::Lore),
                1,
                "Extra lore".into(),
            ),
        ),
        (
            "Double lore",
            StaticAbility::double_counters_replacement(
                ObjectFilter::default(),
                Some(CounterType::Lore),
                "Double lore".into(),
            ),
        ),
    ] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(ability));
        replacements.push(source);
    }
    game.turn.active_player = alice;
    let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMain);
    let mut tq = TriggerQueue::new();
    for _ in &sagas {
        let mut selected = None;
        for _ in 0..2 {
            let TurnAction::Decision(DecisionContext::SelectOptions(ctx)) =
                runner.advance(&mut game, &mut tq).unwrap()
            else {
                panic!("each Saga's lore replacement must be chosen");
            };
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.options.iter().filter(|option| option.legal).count(), 2);
            let second = ctx
                .options
                .iter()
                .find(|option| option.object_id == Some(replacements[1]))
                .unwrap()
                .index;
            assert_ne!(
                second,
                ctx.options
                    .iter()
                    .find(|option| option.legal)
                    .unwrap()
                    .index
            );
            assert_eq!(*selected.get_or_insert(second), second);
            for &saga in &sagas {
                assert_eq!(game.counter_count(saga, CounterType::Lore), 0);
            }
            assert!(
                tq.is_empty(),
                "partial chapter triggers must not escape replay"
            );
        }
        runner.respond_options(vec![selected.unwrap()]);
    }
    assert!(matches!(
        runner.advance(&mut game, &mut tq).unwrap(),
        TurnAction::RunPriority
    ));
    assert!(!runner.has_pending_replay_choice());
    for saga in sagas {
        assert_eq!(
            game.counter_count(saga, CounterType::Lore),
            3,
            "double before adding one"
        );
    }
    assert_eq!(
        tq.entries.len(),
        6,
        "chapters I, II, III trigger once for each Saga"
    );
}
