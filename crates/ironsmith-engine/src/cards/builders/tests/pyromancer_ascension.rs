#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn pyromancer_ascension_keeps_cast_time_name_relation_and_source_threshold() {
    let name = "Pyromancer Ascension";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [same_name, threshold] = triggered.as_slice() else {
        panic!(
            "expected exactly two triggered abilities: {:#?}",
            definition.abilities
        );
    };

    let same_name_trigger = same_name
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("first ability should use a spell-cast trigger");
    assert_eq!(
        same_name_trigger.filter,
        Some(crate::filter::ObjectFilter::instant_or_sorcery())
    );
    assert_eq!(
        same_name_trigger.same_name_card_in_zone,
        Some((Zone::Graveyard, PlayerFilter::You))
    );
    assert!(same_name.intervening_if.is_none());

    let threshold_trigger = threshold
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("second ability should use a spell-cast trigger");
    assert_eq!(
        threshold_trigger,
        &crate::triggers::SpellCastTrigger::new(
            Some(crate::filter::ObjectFilter::instant_or_sorcery()),
            PlayerFilter::You,
        )
    );
    assert!(matches!(
        threshold.intervening_if,
        Some(crate::effect::Condition::SourceHasCounterAtLeast {
            counter_type: crate::object::CounterType::Quest,
            count: 2,
            ..
        })
    ));

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
