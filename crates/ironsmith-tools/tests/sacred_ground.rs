//! Sacred Ground: "Whenever a spell or ability an opponent controls causes a
//! land to be put into your graveyard from the battlefield, return that card
//! to the battlefield."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::{CardId, StableId};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Supertype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Sacred Ground",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

fn card(name: &str, card_type: CardType) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Land {
        builder = builder.supertypes(vec![Supertype::Basic]);
    }
    if card_type == CardType::Creature {
        builder = builder.power_toughness(ironsmith::card::PowerToughness::fixed(2, 2));
    }
    builder.build()
}

/// Alice controls Sacred Ground and `victim`; a source controlled by
/// `destroyer` destroys it. Returns the zone the victim card ends up in once
/// any triggers resolve.
fn destroy(victim_type: CardType, destroyer: PlayerId) -> Zone {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let victim = game.create_object_from_definition(&card("Victim", victim_type), alice, Zone::Battlefield);
    let victim_stable: StableId = game.object(victim).unwrap().stable_id;
    let source: ObjectId =
        game.create_object_from_definition(&card("Destroyer", CardType::Artifact), destroyer, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, destroyer, &mut dm);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::destroy(ChooseSpec::SpecificObject(victim)),
        &mut ctx,
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
    let id = game.find_object_by_stable_id(victim_stable).unwrap();
    let object = game.object(id).unwrap();
    if object.zone == Zone::Battlefield {
        assert_eq!(game.controller_of_id(id), Some(alice), "returns under its owner's control");
    }
    object.zone
}

#[test]
fn an_opponents_destruction_of_your_land_is_undone() {
    assert_eq!(destroy(CardType::Land, PlayerId::from_index(1)), Zone::Battlefield);
}

#[test]
fn your_own_effects_do_not_trigger_it() {
    assert_eq!(destroy(CardType::Land, PlayerId::from_index(0)), Zone::Graveyard);
}

#[test]
fn only_lands_return() {
    assert_eq!(destroy(CardType::Creature, PlayerId::from_index(1)), Zone::Graveyard);
}
