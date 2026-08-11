#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{parse_oracle_card_definition, resolve_triggers_for_source};
use super::*;
use crate::decision::{AutoPassDecisionMaker, LegalAction, SelectFirstDecisionMaker};
use crate::effects::ExecutionContext;
use crate::mana::{ManaCost, ManaSymbol};

const ORACLE_TEXT: &str = "Trample\nWhenever you cast a spell with mana value 4 or greater, you may exile the top card of your library. If you do, you may play that card until you exile another card with this creature.";

fn spell(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn free_spell(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn cast_event(spell: ObjectId, caster: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    )
}

fn resolve_accepting_trigger_for_source(
    game: &mut crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let matching = crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    let count = matching.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in matching {
        queue.add(trigger);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("Superior Foes trigger should go on the stack");
        crate::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker)
            .expect("accepted Superior Foes trigger should resolve");
    }
    count
}

fn current_id(game: &crate::GameState, stable_id: StableId) -> ObjectId {
    game.find_object_by_stable_id(stable_id)
        .expect("the stable card should remain in the game")
}

fn can_play_from_exile(game: &crate::GameState, card: ObjectId, player: PlayerId) -> bool {
    game.effect_store
        .grant_registry
        .card_can_play_from_zone(game, card, Zone::Exile, player)
}

fn has_cast_action(game: &crate::GameState, card: ObjectId, player: PlayerId) -> bool {
    crate::decision::compute_legal_actions(game, player)
        .iter()
        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == card))
}

#[test]
fn superior_foes_named_trigger_tracks_card_owner_identity_and_same_source_expiry() {
    let definition = parse_oracle_card_definition("Superior Foes of Spider-Man");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE_TEXT
    );
    let ability_debug = format!("{:#?}", definition.abilities);
    assert!(
        ability_debug.contains("mana_value: Some")
            && ability_debug.contains("GreaterThanOrEqual")
            && ability_debug.contains("UntilSourceExilesAnother")
            && ability_debug.contains("allow_land: true"),
        "Superior Foes must retain its threshold and source-linked play grant: {ability_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let alice_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);

    let alice_next =
        game.create_object_from_definition(&free_spell("Alice Next Exile"), alice, Zone::Library);
    let alice_first = game.create_object_from_definition(
        &free_spell("Shared-Name Permission Card"),
        alice,
        Zone::Library,
    );
    let bob_first =
        game.create_object_from_definition(&free_spell("Bob Permission Card"), bob, Zone::Library);
    let bob_same_name = game.create_object_from_definition(
        &free_spell("Shared-Name Permission Card"),
        bob,
        Zone::Exile,
    );
    let alice_first_stable = game
        .object(alice_first)
        .expect("Alice top exists")
        .stable_id;
    let alice_next_stable = game
        .object(alice_next)
        .expect("Alice next exists")
        .stable_id;
    let bob_first_stable = game.object(bob_first).expect("Bob top exists").stable_id;
    assert!(game.set_player_library_order_with_audit(
        alice,
        vec![alice_next, alice_first],
        "Superior Foes named source-link regression setup",
    ));
    assert!(game.set_player_library_order_with_audit(
        bob,
        vec![bob_first],
        "Superior Foes other-source regression setup",
    ));

    let alice_small_spell =
        game.create_object_from_definition(&spell("Alice MV3 Spell", 3), alice, Zone::Stack);
    assert_eq!(
        resolve_triggers_for_source(
            &mut game,
            alice_source,
            &cast_event(alice_small_spell, alice),
        ),
        0,
        "a mana-value-3 spell must not trigger Superior Foes"
    );
    assert_eq!(
        game.object(alice_first).expect("top card remains").zone,
        Zone::Library
    );

    let alice_big_spell =
        game.create_object_from_definition(&spell("Alice MV4 Spell", 4), alice, Zone::Stack);
    assert_eq!(
        resolve_accepting_trigger_for_source(
            &mut game,
            alice_source,
            &cast_event(alice_big_spell, alice),
        ),
        1,
        "Alice's mana-value-4 spell must trigger her Superior Foes once"
    );
    let alice_first_exiled = current_id(&game, alice_first_stable);
    let first = game
        .object(alice_first_exiled)
        .expect("Alice first card should remain addressable");
    assert_eq!(first.zone, Zone::Exile);
    assert_eq!(first.owner, alice, "exile must preserve the card's owner");
    assert_eq!(first.name, "Shared-Name Permission Card");
    assert_eq!(
        game.object(alice_next).expect("next card remains").zone,
        Zone::Library
    );
    assert!(can_play_from_exile(&game, alice_first_exiled, alice));
    assert!(has_cast_action(&game, alice_first_exiled, alice));
    assert!(!can_play_from_exile(&game, alice_first_exiled, bob));
    assert!(
        !can_play_from_exile(&game, bob_same_name, alice),
        "the grant must follow exact stable card identity, not a same-name card owned by Bob"
    );

    let bob_big_spell =
        game.create_object_from_definition(&spell("Bob MV4 Spell", 4), bob, Zone::Stack);
    assert_eq!(
        resolve_accepting_trigger_for_source(
            &mut game,
            bob_source,
            &cast_event(bob_big_spell, bob),
        ),
        1,
        "Bob's spell must trigger only Bob's separately controlled copy"
    );
    let bob_first_exiled = current_id(&game, bob_first_stable);
    assert_eq!(
        game.object(bob_first_exiled)
            .expect("Bob exiled card")
            .owner,
        bob
    );
    assert!(can_play_from_exile(&game, bob_first_exiled, bob));
    assert!(!can_play_from_exile(&game, bob_first_exiled, alice));
    assert!(
        can_play_from_exile(&game, alice_first_exiled, alice),
        "another source's exile must not expire Alice's permission"
    );

    let alice_second_big_spell =
        game.create_object_from_definition(&spell("Alice Second MV4 Spell", 4), alice, Zone::Stack);
    assert_eq!(
        resolve_accepting_trigger_for_source(
            &mut game,
            alice_source,
            &cast_event(alice_second_big_spell, alice),
        ),
        1
    );
    let alice_next_exiled = current_id(&game, alice_next_stable);
    assert_eq!(
        game.object(alice_next_exiled)
            .expect("next exiled card")
            .zone,
        Zone::Exile
    );
    assert!(
        !can_play_from_exile(&game, alice_first_exiled, alice),
        "the same source's next successful exile must expire the old permission"
    );
    assert!(!has_cast_action(&game, alice_first_exiled, alice));
    assert!(can_play_from_exile(&game, alice_next_exiled, alice));
    assert!(has_cast_action(&game, alice_next_exiled, alice));
    assert!(
        can_play_from_exile(&game, bob_first_exiled, bob),
        "Alice's source must not expire Bob's separately sourced permission"
    );
}

#[test]
fn superior_foes_may_decline_the_exile_without_creating_permission() {
    let definition = parse_oracle_card_definition("Superior Foes of Spider-Man");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Superior Foes should have its spell-cast trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let top = game.create_object_from_definition(
        &free_spell("Declined Superior Foes Card"),
        alice,
        Zone::Library,
    );
    let mut decisions = AutoPassDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("declined Superior Foes trigger should resolve");
    assert_eq!(
        game.object(top).expect("declined top card").zone,
        Zone::Library
    );
    assert!(!can_play_from_exile(&game, top, alice));
}
