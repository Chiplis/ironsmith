#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

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
fn elspeth_resplendent_keeps_the_fixed_counter_and_four_kind_choice() {
    let definition = parse_oracle_card_definition("Elspeth Resplendent");
    assert_exact_oracle("Elspeth Resplendent", &definition);

    let mut fixed = None;
    let mut choice = None;
    for effect in definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .flat_map(|program| program.flattened_default_effects())
    {
        fixed = fixed.or_else(|| find_nested::<crate::effects::PutCountersEffect>(effect));
        choice = choice.or_else(|| find_nested::<crate::effects::ChooseModeEffect>(effect));
    }
    let fixed = fixed.expect("+1 should put the fixed +1/+1 counter");
    let choice = choice.expect("+1 should retain the counter-kind choice");
    assert_eq!(fixed.counter_type, crate::CounterType::PlusOnePlusOne);
    assert_eq!(choice.modes.len(), 4);
    let kinds = choice
        .modes
        .iter()
        .map(|mode| {
            let [effect] = mode.effects.as_slice() else {
                panic!("each counter choice must contain exactly one effect: {mode:#?}");
            };
            effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .expect("counter mode")
                .counter_type
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            crate::CounterType::Flying,
            crate::CounterType::FirstStrike,
            crate::CounterType::Lifelink,
            crate::CounterType::Vigilance,
        ]
    );
}

#[test]
fn archivist_uses_the_event_time_monarch_end_step_surface() {
    let definition = parse_oracle_card_definition("Archivist of Gondor");
    assert_exact_oracle("Archivist of Gondor", &definition);

    let end_step = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>(
            ),
            _ => None,
        })
        .find(|trigger| trigger.surface == ironsmith_core::trigger_model::EndStepSurface::Monarch)
        .expect("the second ability must retain the monarch event qualification");
    assert_eq!(end_step.player, PlayerFilter::Any);
}

#[test]
fn abuelos_awakening_keeps_owned_graveyard_and_x_entry_counters() {
    let definition = parse_oracle_card_definition("Abuelo's Awakening");
    assert_exact_oracle("Abuelo's Awakening", &definition);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Awakening should compile as a spell program");
    let returned = program
        .flattened_default_effects()
        .iter()
        .find_map(find_nested::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>)
        .expect("the first sentence should remain a typed graveyard return");
    let ChooseSpec::Object(filter) = returned.target.base() else {
        panic!("expected one typed target filter: {returned:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    let [counter] = returned.enters_with_counters.as_slice() else {
        panic!("X counters must be part of the battlefield entry: {returned:#?}");
    };
    assert_eq!(counter.counter_type, crate::CounterType::PlusOnePlusOne);
    assert!(matches!(counter.amount.unhinted(), crate::effect::Value::X));
}
