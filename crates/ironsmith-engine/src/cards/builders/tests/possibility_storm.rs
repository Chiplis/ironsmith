#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::triggers::TriggerMatcher;

fn cast_trigger(
    definition: &CardDefinition,
) -> (&TriggeredAbility, &crate::triggers::SpellCastTrigger) {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|matcher| (triggered, matcher)),
            _ => None,
        })
        .expect("Possibility Storm should retain its spell-cast trigger")
}

fn spell_definition(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .with_spell_effect(vec![Effect::draw(1)])
        .build()
}

#[test]
fn possibility_storm_keeps_cast_origin_consult_and_source_exiled_cleanup() {
    let definition = parse_oracle_card_definition("Possibility Storm");
    let oracle = &oracle_text_by_name()["Possibility Storm"];
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle.as_str(),
        "{:#?}",
        cast_trigger(&definition).0.effects,
    );

    let (triggered, matcher) = cast_trigger(&definition);
    assert_eq!(matcher.caster, PlayerFilter::Any);
    let filter = matcher
        .filter
        .as_ref()
        .expect("the trigger must retain its hand origin");
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::IteratedPlayer),
        "actor-relative `their hand` must bind the spell owner to the caster"
    );

    let [segment] = triggered.effects.segments.as_slice() else {
        panic!(
            "the correlated process should remain one segment: {:#?}",
            triggered.effects
        );
    };
    let [tag_triggering, tagged_exile, consult, may_cast, cleanup] =
        segment.default_effects.as_slice()
    else {
        panic!("expected the exact five-effect process: {segment:#?}");
    };
    assert!(
        tag_triggering
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    );
    assert!(
        tagged_exile
            .downcast_ref::<crate::effects::TaggedEffect>()
            .and_then(|tagged| tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>())
            .is_some_and(|moved| moved.zone == Zone::Exile)
    );
    let consult = consult
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
        .expect("the library traversal must remain executable");
    assert_eq!(consult.player, PlayerFilter::IteratedPlayer);
    assert_eq!(consult.mode, ironsmith_core::LibraryConsultMode::Exile);
    assert!(consult.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
    }));
    assert!(
        may_cast
            .downcast_ref::<crate::effects::MayEffect>()
            .is_some_and(|may| may.decider == Some(PlayerFilter::IteratedPlayer))
    );
    assert!(
        cleanup
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|moved| {
                moved.zone == Zone::Library
                    && !moved.to_top
                    && moved.library_order == Some(ironsmith_core::LibraryPlacementOrder::Random)
                    && moved.exiled_with_source_surface.is_some()
            })
    );
}

#[test]
fn possibility_storm_casts_only_the_matching_card_and_bottoms_the_source_exiled_remainder() {
    let definition = parse_oracle_card_definition("Possibility Storm");
    let (triggered, matcher) = cast_trigger(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let triggering = game.create_object_from_definition(
        &spell_definition("Triggering Sorcery", CardType::Sorcery),
        bob,
        Zone::Stack,
    );
    let triggering_stable = game.object(triggering).expect("triggering spell").stable_id;
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            triggering,
            bob,
            Zone::Hand,
            crate::snapshot::ObjectSnapshot::from_object(
                game.object(triggering).expect("triggering spell"),
                &game,
            ),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(matcher.matches(
        &event,
        &crate::triggers::TriggerContext::for_source(source, alice, &game),
    ));
    let another_players_spell = game.create_object_from_definition(
        &spell_definition("Another Player's Sorcery", CardType::Sorcery),
        alice,
        Zone::Stack,
    );
    let wrong_owner = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(another_players_spell, bob, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        !matcher.matches(
            &wrong_owner,
            &crate::triggers::TriggerContext::for_source(source, alice, &game),
        ),
        "a spell cast from another player's hand must not satisfy `their hand`"
    );
    let nonhand = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(triggering, bob, Zone::Exile),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        !matcher.matches(
            &nonhand,
            &crate::triggers::TriggerContext::for_source(source, alice, &game),
        ),
        "the same spell cast from outside its caster's hand must not trigger"
    );

    let matching = game.create_object_from_definition(
        &spell_definition("Matching Sorcery", CardType::Sorcery),
        bob,
        Zone::Library,
    );
    let matching_stable = game.object(matching).expect("matching card").stable_id;
    let nonmatching = game.create_object_from_definition(
        &spell_definition("Nonmatching Creature", CardType::Creature),
        bob,
        Zone::Library,
    );
    let nonmatching_stable = game
        .object(nonmatching)
        .expect("nonmatching card")
        .stable_id;
    let unrelated = game.create_object_from_definition(
        &spell_definition("Unrelated Exiled Card", CardType::Sorcery),
        bob,
        Zone::Exile,
    );

    let mut decisions = SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Possibility Storm trigger should resolve");

    let matching = game
        .find_object_by_stable_id(matching_stable)
        .and_then(|id| game.object(id))
        .expect("matching spell remains tracked");
    assert_eq!(matching.zone, Zone::Stack);
    assert_eq!(game.stack.last().map(|entry| entry.controller), Some(bob));
    assert_eq!(
        game.find_object_by_stable_id(triggering_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Library),
        "the original spell belongs in the random-bottom remainder"
    );
    assert_eq!(
        game.find_object_by_stable_id(nonmatching_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Library),
        "a nonmatching consulted card belongs in the random-bottom remainder"
    );
    assert_eq!(
        game.object(unrelated)
            .expect("unrelated exile remains")
            .zone,
        Zone::Exile,
        "cleanup must consume only cards exiled with this source"
    );
}
