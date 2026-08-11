#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::alternative_cast::CastingMethod;
use crate::filter::StackObjectKind;
use crate::game_state::{StackEntry, Target};

fn nested_counter(effect: &Effect) -> Option<crate::effects::CounterEffect> {
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return Some(counter.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_counter(child);
        }
    });
    found
}

fn push_spell_from_zone(
    game: &mut crate::GameState,
    controller: PlayerId,
    name: &str,
    from_zone: Zone,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    let origin_id = game.create_object_from_definition(&definition, controller, from_zone);
    let stack_id = game
        .move_object_by_effect(origin_id, Zone::Stack)
        .expect("probe spell should move to the stack");
    let casting_method = if from_zone == Zone::Hand {
        CastingMethod::Normal
    } else {
        CastingMethod::PlayFrom {
            source: stack_id,
            zone: from_zone,
            use_alternative: None,
        }
    };
    game.push_to_stack(StackEntry::new(stack_id, controller).with_casting_method(casting_method));
    let cast_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(stack_id, controller, from_zone),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&cast_event);
    stack_id
}

#[test]
fn laquatus_disdain_keeps_cast_origin_in_structure_text_and_target_legality() {
    let definition = parse_oracle_card_definition("Laquatus's Disdain");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Counter target spell cast from a graveyard.\nDraw a card."
    );

    let counter = definition
        .spell_effect
        .as_ref()
        .expect("Laquatus's Disdain should have a spell program")
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_counter)
        .expect("Laquatus's Disdain should contain a counter effect");
    let ChooseSpec::Object(filter) = counter.target.base() else {
        panic!("counter target should be an object filter: {counter:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
    assert_eq!(
        filter.stack_kind,
        Some(StackObjectKind::Spell),
        "{filter:#?}"
    );
    assert_eq!(filter.cast_by, None, "{filter:#?}");

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let graveyard_card_definition = CardDefinitionBuilder::new(CardId::new(), "Graveyard Card")
        .card_types(vec![CardType::Instant])
        .build();
    let graveyard_card =
        game.create_object_from_definition(&graveyard_card_definition, bob, Zone::Graveyard);
    let graveyard_spell = push_spell_from_zone(&mut game, bob, "Graveyard Spell", Zone::Graveyard);
    let hand_spell = push_spell_from_zone(&mut game, bob, "Hand Spell", Zone::Hand);

    let legal =
        crate::game_loop::compute_legal_targets(&game, &counter.target, alice, Some(source));
    assert!(
        legal.contains(&Target::Object(graveyard_spell)),
        "{legal:#?}"
    );
    assert!(!legal.contains(&Target::Object(hand_spell)), "{legal:#?}");
    assert!(
        !legal.contains(&Target::Object(graveyard_card)),
        "an ordinary graveyard card is not a stack spell: {legal:#?}"
    );
}
