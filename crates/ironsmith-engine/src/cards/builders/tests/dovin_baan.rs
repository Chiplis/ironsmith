#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::ActivatedAbility;

const ORACLE: &str = "+1: Until your next turn, up to one target creature gets -3/-0 and its activated abilities can't be activated.\n−1: You gain 2 life and draw a card.\n−7: You get an emblem with \"Your opponents can't untap more than two permanents during their untap steps.\"";

fn first_loyalty_ability(definition: &CardDefinition) -> &ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability() => Some(activated),
            _ => None,
        })
        .expect("Dovin Baan should have a +1 loyalty ability")
}

#[test]
fn dovin_baan_keeps_the_shared_next_turn_duration() {
    let definition = parse_oracle_card_definition("Dovin Baan");
    let compiled = canonical_compiled_lines(&definition).join("\n");

    let activated = first_loyalty_ability(&definition);
    assert_eq!(activated.choices.len(), 1, "{activated:#?}");
    let [sequence] = activated.effects.flattened_default_effects() else {
        panic!("expected one coordinated duration sequence: {activated:#?}");
    };
    let sequence = sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the shared leading duration should remain coordinated");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
    );

    let mut pump = None;
    let mut restriction = None;
    fn collect(
        effect: &Effect,
        pump: &mut Option<crate::effects::ApplyContinuousEffect>,
        restriction: &mut Option<crate::effects::CantEffect>,
    ) {
        if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
            *pump = Some(apply.clone());
        }
        if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
            *restriction = Some(cant.clone());
        }
        effect.visit_child_effects(&mut |child| collect(child, pump, restriction));
    }
    for effect in &sequence.effects {
        collect(effect, &mut pump, &mut restriction);
    }
    let pump = pump.expect("the +1 should keep its power modifier");
    assert_eq!(pump.until, crate::effect::Until::YourNextTurn);
    let restriction = restriction.expect("the +1 should keep its activation restriction");
    assert_eq!(restriction.duration, crate::effect::Until::YourNextTurn);
    assert_eq!(
        restriction.duration_surface,
        crate::effect::RestrictionDurationSurface::LeadingUntilYourNextTurn
    );
    assert_eq!(compiled, ORACLE);
}
