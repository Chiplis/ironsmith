use super::*;
const TEXT: &str = "When you unlock this door, manifest dread, then put two +1/+1 counters and a trample counter on that creature.";

#[test]
fn manifest_dread_counter_bundle_room_affects_only_the_new_creature() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Experimental Lab")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let room = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
    for _ in 0..2 {
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::UnlockDoor,
            alice,
            room,
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(triggers.len(), 1);
    let mut ctx =
        crate::effects::EffectContext::new_default(room, alice).with_triggering_event(event);
    for segment in &triggers[0].ability.effects.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    game.refresh_continuous_state();
    let new_creatures = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| *id != room && *id != other)
        .collect::<Vec<_>>();
    assert_eq!(new_creatures.len(), 1);
    let manifested = new_creatures[0];
    assert!(game.is_face_down(manifested));
    assert_eq!(
        game.counter_count(manifested, CounterType::PlusOnePlusOne),
        2
    );
    assert_eq!(game.counter_count(manifested, CounterType::Trample), 1);
    assert_eq!(game.current_power(manifested), Some(4));
    assert!(game.current_has_static_ability_id(
        manifested,
        crate::static_abilities::StaticAbilityId::Trample
    ));
    assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 0);
    assert_eq!(game.counter_count(other, CounterType::Trample), 0);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    assert!(game.player(alice).unwrap().library.is_empty());
}

#[test]
fn manifest_dread_counter_bundle_room_renders_the_shared_target() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Experimental Lab")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn manifest_dread_counter_bundle_requires_a_shared_recipient() {
    let first = Effect::new(crate::effects::TaggedEffect {
        tag: "counter_recipient".into(),
        effect: Box::new(Effect::put_counters(CounterType::PlusOnePlusOne, 2, ChooseSpec::Source)),
        outcome_only: false,
    });
    let second = crate::effects::PutCountersEffect {
        counter_type: CounterType::Trample,
        amount: Value::Fixed(1),
        target: ChooseSpec::tagged("counter_recipient"),
        target_count: None,
        distributed: false,
    };
    let render = |second| {
        super::super::effect_lists::describe_shared_recipient_counter_pair(&[
            first.clone(),
            Effect::new(second),
        ])
    };
    assert!(render(second.clone()).is_some());
    let mut unrelated = second.clone();
    unrelated.target = ChooseSpec::tagged("other_recipient");
    assert!(render(unrelated).is_none());
    let mut distributed = second;
    distributed.distributed = true;
    assert!(render(distributed).is_none());
}
