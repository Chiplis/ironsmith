#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effect::{Effect, Value};

#[test]
fn target_and_value_correlation_cards_render_exactly() {
    for (name, oracle) in [
        (
            "Dragonspark Reactor",
            "Whenever this artifact or another artifact you control enters, put a charge counter on this artifact.\n{4}, Sacrifice this artifact: It deals damage equal to the number of charge counters on it to target player and that much damage to up to one target creature.",
        ),
        (
            "Wisecrack",
            "Target creature deals damage equal to its power to itself. If that creature is attacking, Wisecrack deals 2 damage to that creature's controller.",
        ),
        (
            "Rin and Seri, Inseparable",
            "Whenever you cast a Dog spell, create a 1/1 green Cat creature token.\nWhenever you cast a Cat spell, create a 1/1 white Dog creature token.\n{R}{G}{W}, {T}: Rin and Seri deals damage to any target equal to the number of Dogs you control. You gain life equal to the number of Cats you control.",
        ),
        (
            "Rite of Consumption",
            "As an additional cost to cast this spell, sacrifice a creature.\nRite of Consumption deals damage equal to the sacrificed creature's power to target player or planeswalker. You gain life equal to the damage dealt this way.",
        ),
        (
            "Thorin, Mountain-king",
            "Trample\nWhen Thorin enters, attach any number of target Equipment you control to target creature you control. When one or more Equipment become attached to that creature this way, that creature deals damage equal to its power to up to one target creature.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert_eq!(compiled, oracle, "{name}: {definition:#?}");
    }
}

#[test]
fn attach_destination_identity_is_exported_to_the_power_damage_source() {
    let definition = parse_oracle_card_definition("Thorin, Mountain-king");
    let debug = format!("{definition:#?}");

    fn uses_attachment_destination_as_power_source(effect: &Effect) -> bool {
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && let ChooseSpec::Tagged(source_tag) = with_source.source.unhinted()
            && source_tag.as_str().starts_with("attachment_target_")
            && let Some(damage) = with_source
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
            && matches!(
                damage.amount.unhinted(),
                Value::PowerOf(power_source)
                    if power_source.unhinted() == &ChooseSpec::Tagged(source_tag.clone())
            )
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= uses_attachment_destination_as_power_source(child);
        });
        found
    }

    let AbilityKind::Triggered(triggered) = &definition.abilities[1].kind else {
        panic!("expected Thorin's second ability to be triggered: {debug}");
    };
    let found = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .any(uses_attachment_destination_as_power_source);

    assert!(debug.contains("attachment_target_"), "{debug}");
    assert!(found, "{debug}");
}
