#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_next_batch_counter_registration(
    effect: &crate::effect::Effect,
) -> Option<crate::effects::RegisterNextBatchEnterWithCountersEffect> {
    if let Some(register) =
        effect.downcast_ref::<crate::effects::RegisterNextBatchEnterWithCountersEffect>()
    {
        return Some(register.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_next_batch_counter_registration(child);
        }
    });
    found
}

#[test]
fn storyweave_keeps_next_simultaneous_etb_batch_registration() {
    let definition = parse_oracle_card_definition("Storyweave");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Choose one —\n• Put two +1/+1 counters on target creature you control.\n• Put two lore counters on target Saga you control. The next time one or more enchantment creatures you control enter this turn, each enters with two additional +1/+1 counters on it."
    );

    let modal = definition
        .spell_effect
        .as_ref()
        .expect("Storyweave should have spell effects")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        .expect("Storyweave should retain its two typed modes");
    assert_eq!(modal.modes.len(), 2);
    let registration = modal.modes[1]
        .effects
        .iter()
        .find_map(find_next_batch_counter_registration)
        .expect("the Saga mode should register the next matching ETB batch");

    assert_eq!(
        registration.counter_type,
        crate::object::CounterType::PlusOnePlusOne
    );
    assert_eq!(registration.count, ironsmith_core::Value::Fixed(2));
    assert_eq!(registration.filter.zone, Some(Zone::Battlefield));
    assert_eq!(registration.filter.controller, Some(PlayerFilter::You));
    assert_eq!(
        registration.filter.all_card_types,
        [CardType::Enchantment, CardType::Creature]
    );
}
