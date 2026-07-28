#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn dynamic_entry_counter_grants_render_the_exact_typed_entry_clause() {
    for (name, expected) in [
        (
            "Communal Brewing",
            "Whenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of ingredient counters on this enchantment.",
        ),
        (
            "Runadi, Behemoth Caller",
            "Whenever you cast a creature spell with mana value 5 or greater, that creature enters with X additional +1/+1 counters on it, where X is its mana value minus 4.",
        ),
        (
            "Wildgrowth Archaic",
            "Whenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of colors of mana spent to cast it.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let lines = unprocessed_compiled_lines(&definition);
        assert!(
            lines.iter().any(|line| line == expected),
            "{name} should preserve its exact dynamic entry-counter clause; got {lines:#?}"
        );
    }
}

#[test]
fn runadi_preserves_the_typed_counter_threshold_in_its_haste_filter() {
    let definition = parse_oracle_card_definition("Runadi, Behemoth Caller");
    let lines = unprocessed_compiled_lines(&definition);
    assert!(
        lines.iter().any(|line| {
            line == "Creatures you control with three or more +1/+1 counters on them have haste."
        }),
        "Runadi should retain its three-counter haste threshold; got {lines:#?}"
    );
}

#[test]
fn communal_brewing_preserves_its_target_fanout_and_counter_articles() {
    let definition = parse_oracle_card_definition("Communal Brewing");
    let lines = unprocessed_compiled_lines(&definition);
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        lines.iter().any(|line| {
            line
                == "When this enchantment enters, any number of target opponents each draw a card. Put an ingredient counter on this enchantment, then put an ingredient counter on it for each card drawn this way."
        }),
        "Communal Brewing should retain its complete first enters ability; got {lines:#?}\n{debug}"
    );

    let (triggered, target_choice) = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find_map(|triggered| {
            triggered
                .choices
                .iter()
                .find(|choice| {
                    choice.count() == crate::effect::ChoiceCount::any_number()
                        && matches!(choice.base(), ChooseSpec::Player(PlayerFilter::Opponent))
                })
                .map(|choice| (triggered, choice))
        })
        .expect("the enters trigger should declare any number of target opponents");
    assert_eq!(
        target_choice.count(),
        crate::effect::ChoiceCount::any_number(),
        "the target declaration must retain its authored cardinality"
    );

    let for_players = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::ForPlayersEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<WithIdEffect>()?
                        .effect
                        .downcast_ref::<crate::effects::ForPlayersEffect>()
                })
        })
        .expect("the targeted opponent set should lower to one executable participant fanout");
    assert_eq!(
        for_players.filter,
        PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
    );
    let [draw_effect] = for_players.effects.as_slice() else {
        panic!("each targeted opponent should perform one draw: {for_players:#?}");
    };
    let draw = draw_effect
        .downcast_ref::<DrawCardsEffect>()
        .expect("the per-participant action should remain a typed draw");
    assert_eq!(draw.player, PlayerFilter::IteratedPlayer);
    assert_eq!(draw.count.unhinted(), &crate::effect::Value::Fixed(1));

    let (counter, count) = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| triggered.effects.flattened_default_effects())
        .find_map(|effect| {
            let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
            let crate::continuous::Modification::AddAbility(ability) =
                apply.modification.as_ref()?
            else {
                return None;
            };
            let ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                counter,
                count,
                ..
            } = &ability.compiled_model()?.payload
            else {
                return None;
            };
            Some((counter, count))
        })
        .expect("the creature-cast trigger should grant a typed entry-counter ability");
    assert_eq!(counter, &CounterType::PlusOnePlusOne);
    assert!(
        matches!(
            count.unhinted(),
            crate::effect::Value::CountersOnSource(CounterType::Named("ingredient"))
        ),
        "the granted ability must read ingredient counters on the outer source: {count:#?}"
    );
}
