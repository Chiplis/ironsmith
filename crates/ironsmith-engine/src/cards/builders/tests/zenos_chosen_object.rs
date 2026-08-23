#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const FRONT_LINES: &[&str] = &[
    "My First Friend — When Zenos yae Galvus enters, choose a creature an opponent controls. Until end of turn, creatures other than Zenos yae Galvus and the chosen creature get -2/-2.",
    "When the chosen creature leaves the battlefield, transform Zenos yae Galvus.",
];

fn linked_zenos_pair() -> (CardDefinition, CardDefinition) {
    let mut zenos = parse_oracle_card_definition("Zenos yae Galvus");
    let mut shinryu = parse_oracle_card_definition("Shinryu, Transcendent Rival");
    let zenos_id = CardId::from_raw(9_870_101);
    let shinryu_id = CardId::from_raw(9_870_102);

    zenos.card.id = zenos_id;
    zenos.card.other_face = Some(shinryu_id);
    zenos.card.other_face_name = Some("Shinryu, Transcendent Rival".to_string());
    zenos.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    shinryu.card.id = shinryu_id;
    shinryu.card.other_face = Some(zenos_id);
    shinryu.card.other_face_name = Some("Zenos yae Galvus".to_string());
    shinryu.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    (zenos, shinryu)
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn zone_change_event(
    snapshot: crate::snapshot::ObjectSnapshot,
    to: Zone,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            snapshot.object_id,
            snapshot.zone,
            to,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn zenos_keeps_the_exact_chosen_creature_program_and_face_name() {
    let (front, back) = linked_zenos_pair();
    assert_eq!(
        canonical_compiled_lines(&front),
        FRONT_LINES
            .iter()
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>()
    );

    let debug = format!("{front:#?}");
    assert!(
        debug.contains("remember_as_chosen_object: true")
            && debug.contains(ironsmith_core::CHOSEN_OBJECTS_TAG)
            && debug.contains("IsNotTaggedObject"),
        "the singular choice, later trigger, and complement must share durable typed identity: {debug}"
    );

    assert_eq!(
        canonical_compiled_lines(&back),
        vec![
            "Flying".to_string(),
            "As this creature transforms into Shinryu, choose an opponent.".to_string(),
            "Burning Chains — When the chosen player loses the game, you win the game.".to_string(),
        ]
    );
}

#[test]
fn chosen_creature_is_excluded_from_debuff_and_is_the_only_leave_trigger() {
    let (mut definition, back_definition) = linked_zenos_pair();
    // The cards.json DFC fixture currently omits the front face's printed P/T
    // from this isolated definition. Install it on the test object so the
    // source-exclusion assertion measures the -2/-2 effect, rather than a
    // missing base characteristic.
    definition.card.power_toughness = Some(PowerToughness::fixed(4, 4));
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.register_linked_face_definition(&definition);
    game.register_linked_face_definition(&back_definition);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let chosen =
        game.create_object_from_definition(&creature("Chosen Rival", 4, 4), bob, Zone::Battlefield);
    let unrelated = game.create_object_from_definition(
        &creature("Unrelated Rival", 4, 4),
        bob,
        Zone::Battlefield,
    );

    let first = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            matches!(
                triggered.presentation_label.as_ref(),
                Some(crate::ability::PresentationLabel::AbilityWord(label))
                    if label.eq_ignore_ascii_case("My First Friend")
            )
            .then_some(triggered)
        })
        .expect("front face must retain the labeled entry trigger");
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &first.effects,
        None,
        &[],
    )
    .expect("entry trigger program should resolve");

    let chosen_memory = game
        .chosen_object(source)
        .expect("the source must persist its exact singular choice");
    assert_eq!(
        chosen_memory.stable_id,
        game.object(chosen)
            .expect("chosen creature exists")
            .stable_id
    );
    assert_eq!(game.current_power(source), Some(4));
    assert_eq!(game.current_power(chosen), Some(4));
    assert_eq!(game.current_power(unrelated), Some(2));

    let unrelated_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(unrelated).expect("unrelated creature exists"),
        &game,
    );
    let unrelated_event = zone_change_event(unrelated_snapshot, Zone::Graveyard);
    assert!(
        crate::triggers::check_triggers(&game, &unrelated_event)
            .into_iter()
            .all(|entry| entry.source != source),
        "an unrelated creature leaving must not transform the source"
    );

    let chosen_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(chosen).expect("chosen creature exists"),
        &game,
    );
    let chosen_stable = chosen_snapshot.stable_id;
    game.move_object_by_effect(chosen, Zone::Graveyard)
        .expect("chosen creature should leave the battlefield");
    let pending_events = game.take_pending_trigger_events();
    let chosen_event = pending_events
        .iter()
        .find(|event| {
            event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .is_some_and(|zone_change| {
                    zone_change.from == Zone::Battlefield
                        && zone_change.to == Zone::Graveyard
                        && zone_change
                            .snapshots()
                            .iter()
                            .any(|snapshot| snapshot.stable_id == chosen_stable)
                })
        })
        .expect("the chosen creature's move must queue its production zone-change event");
    assert!(
        chosen_event
            .lookback_source_snapshots()
            .iter()
            .any(|snapshot| snapshot.object_id == source),
        "the production event must preserve the pre-event trigger source required by CR 603.10"
    );
    let entries = crate::triggers::check_triggers(&game, chosen_event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        1,
        "only the chosen-object trigger should match"
    );

    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("chosen-creature trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("chosen-creature trigger should resolve");
    assert_eq!(
        game.object(source)
            .expect("source remains on battlefield")
            .name
            .as_ref(),
        "Shinryu, Transcendent Rival"
    );
    assert_eq!(
        game.chosen_object(source)
            .map(|snapshot| snapshot.stable_id),
        Some(chosen_stable),
        "in-place transformation must retain the source's chosen-object memory"
    );

    let moved_source = game
        .move_object_by_effect(source, Zone::Graveyard)
        .expect("transformed source should leave the battlefield");
    assert!(game.chosen_object(source).is_none());
    assert!(
        game.chosen_object(moved_source).is_none(),
        "a source becoming a new zone object must not carry battlefield choice memory"
    );
}

#[test]
fn resolution_local_object_choice_without_a_later_reference_is_not_persisted() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Local Chooser")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("When this creature enters, choose a creature an opponent controls.")
        .expect("local choice fixture should parse");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(
        !debug.contains("remember_as_chosen_object: true"),
        "a choice with no cross-ability chosen-object consumer must stay resolution-local: {debug}"
    );
}
