#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const TRIGGER_LINE: &str = "When enchanted land becomes tapped, destroy it. That land's controller may attach this Aura to a land of their choice.";

struct AttachDecision {
    accept: bool,
    chooser: PlayerId,
    destination: ObjectId,
    may_players: Vec<PlayerId>,
}

impl crate::decision::DecisionMaker for AttachDecision {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.may_players.push(ctx.player);
        self.accept
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        assert_eq!(ctx.player, self.chooser);
        assert!(
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == self.destination && candidate.legal),
            "the chosen land must be a legal attachment destination: {ctx:#?}"
        );
        vec![self.destination]
    }
}

fn land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

fn resolve_kudzu_trigger(accept: bool) -> (crate::GameState, ObjectId, ObjectId, AttachDecision) {
    let definition = parse_oracle_card_definition("Kudzu");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let kudzu = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let enchanted =
        game.create_object_from_definition(&land("Enchanted Land"), bob, Zone::Battlefield);
    let enchanted_stable_id = game
        .object(enchanted)
        .expect("enchanted land should exist")
        .stable_id;
    let destination =
        game.create_object_from_definition(&land("Destination Land"), bob, Zone::Battlefield);
    assert!(
        game.attach_object_to_target(kudzu, crate::object::AttachmentTarget::Object(enchanted),)
    );

    game.tap(enchanted);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(enchanted),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &event) {
        queue.add(entry);
    }
    assert_eq!(queue.entries.len(), 1);
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Kudzu trigger should go on the stack");

    let mut decisions = AttachDecision {
        accept,
        chooser: bob,
        destination,
        may_players: Vec::new(),
    };
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Kudzu trigger should resolve");
    let destroyed = game
        .find_object_by_stable_id(enchanted_stable_id)
        .expect("destroyed land should remain tracked across its zone change");
    (game, kudzu, destroyed, decisions)
}

#[test]
fn kudzu_preserves_controller_optional_attachment_structure_and_surface() {
    let definition = parse_oracle_card_definition("Kudzu");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Enchant land".to_string(), TRIGGER_LINE.to_string()]
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Kudzu should have a tapped-land trigger");
    let debug = format!("{:#?}", triggered.effects);
    assert!(
        debug.contains("MayEffect"),
        "attachment must be optional: {debug}"
    );
    assert!(
        debug.contains("ControllerOf")
            && debug.contains("Tagged(\n")
            && debug.contains("\"triggering\""),
        "the destroyed land's controller must decide: {debug}"
    );
}

#[test]
fn kudzu_destroyed_land_controller_can_accept_or_decline_the_attachment() {
    let (accepted_game, kudzu, destroyed, accepted) = resolve_kudzu_trigger(true);
    assert_eq!(accepted.may_players, vec![PlayerId::from_index(1)]);
    assert_eq!(
        accepted_game.object(destroyed).map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert!(matches!(
        accepted_game.object(kudzu).and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Object(target)) if target == accepted.destination
    ));

    let (declined_game, declined_kudzu, destroyed, declined) = resolve_kudzu_trigger(false);
    assert_eq!(declined.may_players, vec![PlayerId::from_index(1)]);
    assert_eq!(
        declined_game.object(destroyed).map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert!(
        !matches!(
            declined_game.object(declined_kudzu).and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(target)) if target == declined.destination
        ),
        "declining must not move the Aura"
    );
}
