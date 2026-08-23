#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::filter::ObjectRef;

fn assert_exact_oracle(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name]
    );
}

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
fn burn_away_watches_the_exact_target_and_exiles_its_controllers_whole_graveyard() {
    let definition = parse_oracle_card_definition("Burn Away");
    assert_exact_oracle("Burn Away", &definition);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Burn Away should have a spell program");
    let schedule = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::ScheduleDelayedTriggerEffect>)
        .expect("Burn Away should register a delayed death watcher");
    assert!(schedule.one_shot);
    assert!(schedule.until_end_of_turn);
    let watched_tag = schedule
        .target_tag
        .as_ref()
        .expect("the delayed watcher should capture the damage target");
    let exile = schedule
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::ExileEffect>)
        .expect("the delayed effect should exile the watched target's graveyard");
    let ChooseSpec::All(filter) = exile.spec.base() else {
        panic!("the entire graveyard must be exiled, got {exile:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert!(matches!(
        &filter.owner,
        Some(PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)))
            if tag == watched_tag || tag.as_str() == "triggering"
    ));
}

#[test]
fn dour_port_mage_keeps_aggregate_non_death_exit_trigger() {
    let definition = parse_oracle_card_definition("Dour Port-Mage");
    assert_exact_oracle("Dour Port-Mage", &definition);

    let zone_change = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>(
            ),
            _ => None,
        })
        .expect("Dour Port-Mage should have a typed zone-change trigger");
    assert_eq!(
        zone_change.from,
        crate::triggers::ZonePattern::Specific(Zone::Battlefield)
    );
    assert_eq!(
        zone_change.to,
        crate::triggers::ZonePattern::AnyExcept(Zone::Graveyard)
    );
    assert_eq!(
        zone_change.count_mode,
        crate::triggers::CountMode::OneOrMore
    );
    assert!(zone_change.object_filter.other);
    assert_eq!(
        zone_change.object_filter.controller,
        Some(PlayerFilter::You)
    );
}

#[test]
fn shalai_subject_union_remains_one_authored_hexproof_line() {
    let definition = parse_oracle_card_definition("Shalai, Voice of Plenty");
    assert_exact_oracle("Shalai, Voice of Plenty", &definition);
    assert!(canonical_compiled_lines(&definition).iter().any(|line| {
        line == "You, planeswalkers you control, and other creatures you control have hexproof."
    }));
}

#[test]
fn wormfang_crab_keeps_opponent_choice_source_exclusion_and_shared_exile_tag() {
    let definition = parse_oracle_card_definition("Wormfang Crab");
    assert_exact_oracle("Wormfang Crab", &definition);

    let enter = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
                .is_some_and(|zone_change| {
                    zone_change.to == crate::triggers::ZonePattern::Specific(Zone::Battlefield)
                })
        })
        .expect("Wormfang Crab should have an enter trigger");
    let effects = enter.effects.flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(find_nested::<crate::effects::ChooseObjectsEffect>)
        .expect("an opponent should choose the permanent to exile");
    assert_eq!(choose.chooser, PlayerFilter::Opponent);
    assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
    assert!(choose.filter.other);
    assert_eq!(
        choose.filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
    let exile = effects
        .iter()
        .find_map(find_nested::<crate::effects::ExileEffect>)
        .expect("the chosen permanent should be exiled");
    assert!(matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag));
}
