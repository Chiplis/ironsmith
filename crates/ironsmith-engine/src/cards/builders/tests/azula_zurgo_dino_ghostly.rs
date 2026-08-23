#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn assert_exact(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name],
        "{definition:#?}"
    );
}

fn assert_compiled(definition: &CardDefinition, expected: &[&str]) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        expected.join("\n"),
        "{definition:#?}"
    );
}

fn find_nested<T: Clone + 'static>(effect: &Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

#[test]
fn azula_keeps_the_dynamic_experience_pump_and_shared_menace_duration() {
    let definition = parse_oracle_card_definition("Azula, Ruthless Firebender");
    assert_compiled(
        &definition,
        &[
            "Firebending 1.",
            "Whenever Azula attacks, you may discard a card. Then you get an experience counter for each player who discarded a card this turn.",
            "{2}{B}: This gets +1/+1 for each experience counter you have until end of turn and this gains menace until end of turn.",
        ],
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if format!("{:#?}", activated.effects).contains("PlayerCounters") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Azula's black activation should remain dynamic");
    let debug = format!("{:#?}", activated.effects);
    assert!(debug.contains("Experience"), "{debug}");
    assert!(debug.contains("Menace"), "{debug}");
    assert!(debug.matches("EndOfTurn").count() >= 2, "{debug}");
}

#[test]
fn zurgo_thunders_decree_keeps_the_conditional_token_restriction() {
    let definition = parse_oracle_card_definition("Zurgo, Thunder's Decree");
    assert_compiled(
        &definition,
        &[
            "Mobilize 2.",
            "During your end step, Warrior tokens you control have \"this token can't be sacrificed\"",
        ],
    );

    let debug = format!("{definition:#?}");
    assert!(debug.contains("SourceControllersEndStep"), "{debug}");
    assert!(debug.contains("Warrior"), "{debug}");
    assert!(debug.contains("token: true"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
}

#[test]
fn dino_dna_keeps_the_source_exiled_target_and_every_copy_exception() {
    let definition = parse_oracle_card_definition("Dino DNA");
    assert_exact("Dino DNA", &definition);

    let copy = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .flat_map(|program| program.flattened_default_effects())
        .find_map(find_nested::<crate::effects::CreateTokenCopyEffect>)
        .expect("Dino DNA should retain the executable copy effect");
    let ChooseSpec::Object(filter) = copy.target.base() else {
        panic!("Dino DNA must target an exiled creature card: {copy:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert_eq!(copy.set_base_power_toughness, Some((6, 6)));
    assert_eq!(copy.set_colors, Some(crate::color::ColorSet::GREEN));
    assert_eq!(copy.set_card_types, Some(vec![CardType::Creature]));
    assert_eq!(copy.set_subtypes, Some(vec![Subtype::Dinosaur]));
    assert_eq!(copy.granted_static_abilities.len(), 1);
    assert_eq!(
        copy.granted_static_abilities[0].id(),
        crate::static_abilities::StaticAbilityId::Trample
    );
}

#[test]
fn ghostly_dancers_keeps_the_resolution_time_room_unlock_choice() {
    let definition = parse_oracle_card_definition("Ghostly Dancers");
    assert_exact("Ghostly Dancers", &definition);

    let unlock = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(&triggered.effects),
            _ => None,
        })
        .flat_map(|program| program.flattened_default_effects())
        .find_map(find_nested::<crate::effects::UnlockRoomDoorEffect>)
        .expect("the ETB choice should retain the executable Room unlock effect");
    assert_eq!(unlock.player, PlayerFilter::You);
    assert_eq!(unlock.room_filter.zone, Some(Zone::Battlefield));
    assert_eq!(unlock.room_filter.controller, Some(PlayerFilter::You));
    assert_eq!(unlock.room_filter.subtypes, vec![Subtype::Room]);
}
