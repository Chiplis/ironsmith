#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "As this enchantment enters, choose Khans or Dragons.\n• Khans — At the beginning of each of your main phases, add {G}{G}.\n• Dragons — Whenever a creature you control with flying enters, you may have it fight target creature you don't control.";

#[test]
fn frontier_siege_keeps_both_typed_named_option_rows() {
    let definition = parse_oracle_card_definition("Frontier Siege");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let khans = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Khans should remain an executable main-phase trigger");
    let phase = khans
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()
        .expect("the Khans trigger should use the typed main-phase matcher");
    assert_eq!(phase.player, PlayerFilter::You);
    assert_eq!(phase.phase_type, crate::triggers::MainPhaseType::Either);
    assert_eq!(
        phase.main_phase_surface,
        ironsmith_core::trigger_model::MainPhaseSurface::EachOfMainPhases
    );
    assert!(matches!(
        khans.intervening_if.as_ref(),
        Some(crate::ConditionExpr::SourceChosenOption(option)) if option == "khans"
    ));
    assert!(khans.effects.segments.iter().any(|segment| {
        segment.default_effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::AddManaEffect>()
                .is_some()
        })
    }));

    let dragons = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if matches!(
                    triggered.intervening_if.as_ref(),
                    Some(crate::ConditionExpr::SourceChosenOption(option)) if option == "dragons"
                ) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Dragons should retain its independently gated trigger");
    assert!(
        dragons
            .trigger
            .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
            .is_some()
    );
}
