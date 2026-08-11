#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const LEAVES_TRIGGER_LINE: &str =
    "When this Aura leaves the battlefield, enchanted permanent's controller sacrifices it.";

#[test]
fn reality_acid_uses_attachment_lki_and_makes_its_controller_sacrifice_it() {
    let definition = parse_oracle_card_definition("Reality Acid");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == LEAVES_TRIGGER_LINE),
        "Reality Acid must preserve attachment and controller attribution: {rendered:#?}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let acid = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let host_definition = CardDefinitionBuilder::new(CardId::new(), "Acid Host")
        .card_types(vec![CardType::Artifact])
        .build();
    let host = game.create_object_from_definition(&host_definition, bob, Zone::Battlefield);
    assert!(game.attach_object_to_target(acid, crate::object::AttachmentTarget::Object(host),));

    let acid_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(acid).expect("Reality Acid exists"),
            &game,
        );
    let acid_stable = acid_snapshot.stable_id;
    let host_stable = game.object(host).expect("enchanted host exists").stable_id;
    game.move_object_by_effect(acid, Zone::Graveyard)
        .expect("Reality Acid should leave the battlefield");

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            acid,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(acid_snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(vec![acid_snapshot]);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Reality Acid should trigger as it leaves"
    );
    assert_eq!(triggers[0].source_stable_id, acid_stable);

    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Reality Acid's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Reality Acid's trigger should resolve");

    let host_zone = game
        .find_object_by_stable_id(host_stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
        .expect("the enchanted permanent should remain represented");
    assert_eq!(
        host_zone,
        Zone::Graveyard,
        "the enchanted permanent must be sacrificed using the Aura's attachment LKI"
    );
}
