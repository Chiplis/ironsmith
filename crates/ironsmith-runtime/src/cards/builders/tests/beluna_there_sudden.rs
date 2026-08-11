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

fn saga_chapter(definition: &CardDefinition, chapter: u32) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.trigger.saga_chapters() == Some(&[chapter]) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("expected Saga chapter")
}

#[test]
fn beluna_keeps_the_permanent_domain_and_adventure_characteristic() {
    let definition = parse_oracle_card_definition("Beluna Grandsquall // Seek Thrills");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Trample\nPermanent spells you cast that have an Adventure cost {1} less to cast.",
        "{definition:#?}",
    );

    let reduction = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("typed Adventure permanent-spell reduction");
    assert!(reduction.filter.has_all_permanent_card_types());
    assert_eq!(reduction.filter.subtypes, [Subtype::Adventure]);
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
}

#[test]
fn there_and_back_again_folds_the_optional_target_and_keeps_token_rule_surface() {
    let definition = parse_oracle_card_definition("There and Back Again");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["There and Back Again"],
        "{definition:#?}",
    );

    let first = saga_chapter(&definition, 1);
    let first_debug = format!("{:#?}", first.effects);
    assert!(first_debug.contains("ChoiceCount {\n"), "{first_debug}");
    assert!(
        first_debug.contains("YouStopControllingThis"),
        "{first_debug}"
    );
    assert!(first_debug.contains("RingTemptsYouEffect"), "{first_debug}");

    let third = saga_chapter(&definition, 3);
    let smaug = third
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::CreateTokenEffect>)
        .expect("chapter III should create the typed Smaug token");
    let death_trigger = smaug
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Smaug should retain its embedded death trigger");
    assert_eq!(
        death_trigger.trigger.intro_surface(),
        Some(crate::triggers::TriggerIntroSurface::When)
    );
    let treasure = death_trigger
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::CreateTokenEffect>)
        .expect("Smaug's death trigger should create Treasure");
    assert_eq!(treasure.count, crate::effect::Value::Fixed(14));
}

#[test]
fn sudden_salvation_preserves_plural_return_and_returned_set_controller_gate() {
    let definition = parse_oracle_card_definition("Sudden Salvation");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Sudden Salvation"],
        "{:#?}",
        definition.spell_effect,
    );
    let program = definition.spell_effect.as_ref().expect("spell program");
    let [target_segment, return_segment, draw_segment] = program.segments.as_slice() else {
        panic!("expected target, return, and correlated draw segments: {program:#?}");
    };

    let targeted = target_segment.default_effects[0]
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("chosen target set should be tagged");
    let returned = return_segment.default_effects[0]
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("returned set should be tagged");
    let move_effect = returned
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("typed return effect");
    assert_eq!(move_effect.target, ChooseSpec::Tagged(targeted.tag.clone()));
    assert!(move_effect.enters_tapped);
    assert_eq!(
        move_effect.battlefield_controller,
        ironsmith_core::BattlefieldController::Owner
    );

    let players = draw_segment.default_effects[0]
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("draw should iterate opponents once each");
    assert_eq!(players.filter, PlayerFilter::Opponent);
    let conditional = players.effects[0]
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("each opponent should be gated by control of the returned set");
    let crate::effect::Condition::PlayerControls { player, filter } = &conditional.condition else {
        panic!("expected typed player-controls condition: {conditional:#?}");
    };
    assert_eq!(player, &PlayerFilter::IteratedPlayer);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == returned.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
}
