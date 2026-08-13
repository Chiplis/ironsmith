#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
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
fn fight_the_blank_fight_targets_the_aura_and_counts_only_long_name_stickers() {
    let definition = parse_oracle_card_definition("Fight the _____ Fight");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Fight the _____ Fight"]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Aura should have its enters trigger");
    let sticker = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_nested::<crate::effects::PutStickerEffect>(effect))
        .expect("enters trigger should contain a sticker action");
    assert_eq!(sticker.target, ChooseSpec::Source);
    assert_eq!(
        sticker.action,
        crate::events::KeywordActionKind::NameSticker
    );

    let anthem_debug = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::Anthem =>
            {
                Some(format!("{static_ability:#?}"))
            }
            _ => None,
        })
        .expect("Aura should grant its sticker-scaled toughness bonus");
    assert!(
        anthem_debug.contains("min_name_letters: Some(8)"),
        "{anthem_debug}"
    );
    assert!(
        anthem_debug.contains("max_name_letters: None"),
        "{anthem_debug}"
    );

    let host_definition = CardDefinitionBuilder::new(CardId::new(), "Sticker Host")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let host = game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(host),));

    assert_eq!(game.calculated_toughness(host), Some(2));
    game.put_name_sticker_on_object(aura, "Sevenss");
    assert_eq!(game.calculated_toughness(host), Some(2));
    game.put_name_sticker_on_object(aura, "Eightsxx");
    assert_eq!(game.calculated_toughness(host), Some(4));
    game.put_name_sticker_on_object(aura, "Ninesxxxx");
    assert_eq!(game.calculated_toughness(host), Some(6));
    assert_eq!(game.calculated_toughness(aura), None);
}
