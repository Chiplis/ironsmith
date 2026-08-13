#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::filter::ObjectFilterExt as _;

fn assert_exact(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name]
    );
}

#[test]
fn reincarnation_binds_the_delayed_return_to_the_watched_creatures_owner() {
    let definition = parse_oracle_card_definition("Reincarnation");
    assert_exact("Reincarnation", &definition);
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("TagTriggeringObjectEffect"), "{debug}");
    assert!(debug.contains("AliasedOwnerOf"), "{debug}");
    assert!(debug.contains("chosen_return"), "{debug}");

    let program = definition.spell_effect.as_ref().expect("spell program");
    let schedule = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("one-shot watched-creature delayed trigger");
    let [payload] = schedule.effects.segments.as_slice() else {
        panic!("expected one delayed payload segment: {schedule:#?}");
    };
    let [tag_triggering, choose_root, _move_root] = payload.default_effects.as_slice() else {
        panic!("expected triggering tag, owner-bound choice, and return: {schedule:#?}");
    };
    let triggering_tag = tag_triggering
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .expect("watched death snapshot tag")
        .tag
        .clone();
    let choose = choose_root
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("owner-bound graveyard choice");

    let creature = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let watched = game.create_object_from_definition(
        &creature("Watched Bob Creature"),
        bob,
        Zone::Battlefield,
    );
    let bob_card =
        game.create_object_from_definition(&creature("Bob Grave Card"), bob, Zone::Graveyard);
    let alice_card =
        game.create_object_from_definition(&creature("Alice Grave Card"), alice, Zone::Graveyard);
    let watched_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(watched).expect("watched creature exists"),
        &game,
    );
    let tagged = std::collections::HashMap::from([(triggering_tag, vec![watched_snapshot])]);
    let context = crate::filter::FilterContext::new(alice).with_tagged_objects(&tagged);
    assert!(
        choose.filter.matches(
            game.object(bob_card).expect("Bob grave card exists"),
            &context,
            &game,
        ),
        "the watched creature's owner may supply the returned creature card"
    );
    assert!(
        !choose.filter.matches(
            game.object(alice_card).expect("Alice grave card exists"),
            &context,
            &game,
        ),
        "a different player's graveyard must not satisfy the owner-bound choice"
    );
}

#[test]
fn generic_graveyard_delayed_return_does_not_invent_watched_owner_binding() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Generic Reincarnation Variant")
        .card_types(vec![crate::types::CardType::Instant])
        .parse_text("Choose target creature. When that creature dies this turn, return a creature card from a graveyard to the battlefield under its owner's control.")
        .expect("generic graveyard delayed return should parse");
    assert!(
        !format!("{definition:#?}").contains("AliasedOwnerOf"),
        "a generic graveyard does not refer to the watched creature's owner"
    );
}

#[test]
fn flame_sweep_keeps_the_complement_of_controlled_fliers() {
    let definition = parse_oracle_card_definition("Flame Sweep");
    assert_exact("Flame Sweep", &definition);
    let program = definition.spell_effect.as_ref().expect("spell program");
    let effects = program.flattened_default_effects();
    let [root] = effects else {
        panic!("expected one typed fanout: {program:#?}");
    };
    let with_id = root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("damage fanout should retain its result identity");
    let for_each = with_id
        .effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .expect("damage should fan out over the exact affected complement");
    assert_eq!(
        for_each.filter.card_types,
        [crate::types::CardType::Creature]
    );
    assert_eq!(for_each.filter.any_of.len(), 2, "{:#?}", for_each.filter);
    assert!(
        for_each
            .filter
            .any_of
            .iter()
            .any(|branch| { branch.controller == Some(PlayerFilter::NotYou) })
    );
    assert!(for_each.filter.any_of.iter().any(|branch| {
        branch
            .excluded_static_abilities
            .contains(&crate::static_abilities::StaticAbilityId::Flying)
    }));
}

#[test]
fn rix_maadi_target_legality_reads_current_turn_life_loss_history() {
    let definition = parse_oracle_card_definition("Rix Maadi Guildmage");
    assert_exact("Rix Maadi Guildmage", &definition);
    let debug = format!("{definition:#?}");
    assert!(debug.contains("LostLifeThisTurn"), "{debug}");
    let qualified = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| format!("{activated:#?}").contains("LostLifeThisTurn"))
        .expect("the second activated ability should retain its history-qualified target");
    assert!(
        qualified
            .choices
            .iter()
            .any(|choice| format!("{choice:#?}").contains("LostLifeThisTurn")),
        "target announcement must use the same qualified player set: {qualified:#?}"
    );

    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let filter = PlayerFilter::lost_life_this_turn(PlayerFilter::Any);
    let context = game.filter_context_for(alice, None);
    assert!(!crate::filter::player_filter_matches_game(
        &filter, bob, &game, &context
    ));
    let life_loss = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::LifeLossEvent::new(bob, 1, false),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&life_loss, None, None);
    let context = game.filter_context_for(alice, None);
    assert!(crate::filter::player_filter_matches_game(
        &filter, bob, &game, &context
    ));
    assert!(!crate::filter::player_filter_matches_game(
        &filter, alice, &game, &context
    ));
}

#[test]
fn an_unqualified_target_player_does_not_gain_the_history_gate() {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(1), "Life Loss Variant")
        .card_types(vec![crate::types::CardType::Creature])
        .parse_text("{B}{R}: Target player loses 1 life.")
        .expect("ordinary target-player life loss should parse");
    let ordinary = PlayerFilter::target_player();
    assert_ne!(
        ordinary,
        PlayerFilter::Target(Box::new(PlayerFilter::lost_life_this_turn(
            PlayerFilter::Any
        )))
    );
    assert!(!format!("{definition:#?}").contains("LostLifeThisTurn"));
}
