#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const MIRROR_OF_FATE_ORACLE: &str = "{T}, Sacrifice this artifact: Choose up to seven face-up exiled cards you own. Exile all the cards from your library, then put the chosen cards on top of your library.";

#[test]
fn mirror_of_fate_renders_the_exact_linked_result_set() {
    let definition = parse_oracle_card_definition("Mirror of Fate");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        MIRROR_OF_FATE_ORACLE
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Mirror of Fate must retain its activated ability");
    let [choice_segment, disposition_segment] = activated.effects.segments.as_slice() else {
        panic!(
            "expected a choice sentence followed by one disposition sentence: {:#?}",
            activated.effects
        );
    };
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        panic!("expected one chosen-set producer: {choice_segment:#?}");
    };
    let choice = choice_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the first sentence must choose cards from exile");
    assert_eq!((choice.count.min, choice.count.max), (0, Some(7)));
    assert_eq!(choice.zone, Some(Zone::Exile));
    assert_eq!(choice.filter.zone, Some(Zone::Exile));
    assert_eq!(choice.filter.owner, Some(PlayerFilter::You));
    assert_eq!(choice.filter.face_down, Some(false));

    let [sequence_effect] = disposition_segment.default_effects.as_slice() else {
        panic!("expected one comma-then disposition: {disposition_segment:#?}");
    };
    let sequence = sequence_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the disposition must retain its typed sequence");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    let [exile_effect, move_effect] = sequence.effects.as_slice() else {
        panic!("expected library exile and chosen-set return: {sequence:#?}");
    };
    let exile = exile_effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .expect("the first disposition action must exile the library");
    assert!(matches!(
        exile.spec.unhinted(),
        ChooseSpec::All(filter)
            if filter.zone == Some(Zone::Library)
                && filter.owner == Some(PlayerFilter::You)
    ));
    let move_to_zone = move_effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("the second disposition action must return the chosen set");
    assert_eq!(move_to_zone.zone, Zone::Library);
    assert!(move_to_zone.to_top);
    assert!(matches!(
        move_to_zone.target.unhinted(),
        ChooseSpec::Tagged(tag) if tag == &choice.tag
    ));
}

#[test]
fn mirror_of_fate_resolves_only_the_chosen_face_up_owned_cards_to_the_new_library() {
    fn test_card(raw_id: u32, name: &str) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(raw_id), name).build()
    }

    let definition = parse_oracle_card_definition("Mirror of Fate");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Mirror of Fate must retain its activated ability");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first_chosen = game.create_object_from_definition(
        &test_card(96_300, "First Face-Up Exiled Card"),
        alice,
        Zone::Exile,
    );
    let second_chosen = game.create_object_from_definition(
        &test_card(96_301, "Second Face-Up Exiled Card"),
        alice,
        Zone::Exile,
    );
    let face_down = game.create_object_from_definition(
        &test_card(96_302, "Face-Down Exiled Card"),
        alice,
        Zone::Exile,
    );
    assert!(game.set_face_down(face_down));
    let opponents_card = game.create_object_from_definition(
        &test_card(96_303, "Opponent-Owned Exiled Card"),
        bob,
        Zone::Exile,
    );
    let old_library = [
        game.create_object_from_definition(
            &test_card(96_304, "Old Library One"),
            alice,
            Zone::Library,
        ),
        game.create_object_from_definition(
            &test_card(96_305, "Old Library Two"),
            alice,
            Zone::Library,
        ),
    ];
    let chosen_stable_ids = [first_chosen, second_chosen].map(|id| {
        game.object(id)
            .expect("chosen exiled card exists")
            .stable_id
    });
    let ineligible_stable_ids = [face_down, opponents_card].map(|id| {
        game.object(id)
            .expect("ineligible exiled card exists")
            .stable_id
    });
    let old_library_stable_ids =
        old_library.map(|id| game.object(id).expect("old library card exists").stable_id);

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Mirror of Fate's linked chosen-set program must resolve");

    let library = &game.player(alice).expect("Alice exists").library;
    assert_eq!(library.len(), 2);
    for stable_id in chosen_stable_ids {
        let chosen = game
            .find_object_by_stable_id(stable_id)
            .expect("chosen card must retain its stable identity");
        assert!(
            library.contains(&chosen),
            "each eligible chosen card must become part of the new library"
        );
    }
    for stable_id in ineligible_stable_ids {
        let ineligible = game
            .find_object_by_stable_id(stable_id)
            .expect("ineligible card must retain its stable identity");
        assert_eq!(
            game.object(ineligible)
                .expect("ineligible card exists")
                .zone,
            Zone::Exile,
            "an ineligible exiled card must remain in exile"
        );
    }
    for stable_id in old_library_stable_ids {
        let formerly_in_library = game
            .find_object_by_stable_id(stable_id)
            .expect("old library card must retain its stable identity");
        assert_eq!(
            game.object(formerly_in_library)
                .expect("old library card exists")
                .zone,
            Zone::Exile,
            "the old library must be exiled before the chosen cards return"
        );
    }
}
