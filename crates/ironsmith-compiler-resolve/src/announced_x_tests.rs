use super::*;
use crate::model::reference_state::ReferenceExports;
use ironsmith_compiler_semantic::model::costs::cost_has_announced_x;
use ironsmith_compiler_semantic::model::CompilerCost;
use ironsmith_core::{ManaCost, ManaSymbol, TotalCost};

#[test]
fn paid_x_survives_both_result_resolution_and_lowering_value_binding() {
    for has_announced_x in [false, true] {
        let env = ReferenceEnv {
            has_announced_x,
            bind_unbound_x_to_last_effect: true,
            last_effect_id: RefState::Known(EffectId(7)),
            ..Default::default()
        };
        let value = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)));
        let expected = Value::Add(
            Box::new(if has_announced_x { Value::X } else { Value::EffectValue(EffectId(7)) }),
            Box::new(Value::Fixed(1)),
        );
        let mut resolved = value.clone();
        resolve_effect_result_value(&mut resolved, effect_reference_resolution_state(&env))
            .expect("resolve branch quantity");
        assert_eq!(resolved, expected);
        assert_eq!(
            crate::reference_helpers::resolve_value_it_tag(&value, &env)
                .expect("lower branch quantity"),
            expected,
        );
    }
}

#[test]
fn paid_x_survives_reference_frames_statement_exports_and_default_configs() {
    let env = ReferenceEnv {
        has_announced_x: true,
        ..Default::default()
    };
    let frame = env.to_lowering_frame(false, false);
    let imports = ReferenceImports::from_lowering_frame(&frame);
    assert!(imports.has_announced_x);
    assert!(!imports.is_empty());
    let restored = ReferenceEnv::from_imports(&imports, false, false, false, None);
    assert!(restored.has_announced_x);
    let exports = ReferenceExports::from_env(&restored);
    assert!(exports.to_imports().has_announced_x);
    assert!(ReferenceExports::join(&exports, &exports).has_announced_x);
    assert!(!ReferenceExports::join(&exports, &ReferenceExports::default()).has_announced_x);

    for (imports, config) in [
        (imports, EffectReferenceResolutionConfig::default()),
        (
            ReferenceImports::default(),
            EffectReferenceResolutionConfig {
                has_announced_x: true,
                ..Default::default()
            },
        ),
    ] {
        let annotated = annotate_effect_sequence(
            &[],
            &imports,
            config,
            IdGenContext::default(),
        )
        .expect("prepare announced-X scope");
        assert!(annotated.final_env.has_announced_x);
    }
}

#[test]
fn paid_x_is_seeded_from_the_actual_cost_including_alternative_payment_branches() {
    let fixed = TotalCost::from_cost(CompilerCost::Mana(ManaCost::from_symbols(vec![
        ManaSymbol::Generic(2),
    ])));
    let variable = TotalCost::from_cost(CompilerCost::Mana(ManaCost::from_symbols(vec![
        ManaSymbol::X,
        ManaSymbol::Red,
    ])));
    assert!(!cost_has_announced_x(&fixed));
    assert!(cost_has_announced_x(&variable));
    assert!(cost_has_announced_x(&TotalCost::one_of(vec![fixed, variable])));
    assert!(cost_has_announced_x(&TotalCost::from_cost(CompilerCost::Life(Value::X))));
    // A receipt-derived "any number" counter payment is not announced mana X.
    assert!(!cost_has_announced_x(&TotalCost::from_cost(CompilerCost::RemoveCounters {
        counter_type: Some(ironsmith_core::CounterType::Charge),
        count: 0,
        filter: None,
        display_x: false,
        dynamic: true,
        single_object: true,
        remove_all: false,
    })));
}
