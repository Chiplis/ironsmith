//! Source-only regression specification. All tests remain UNRUN.
//! These assertions inspect compiler-owned lowering before runtime conversion.
use ironsmith_compiler as compiler;
use compiler::ability::AbilityKind;
use compiler::effect::{Restriction, Until, Value};
use compiler::effects::{AdditionalPhase, AdditionalPhasesEffect, CantEffect, ChooseModeEffect};
use compiler::target::ObjectFilter;

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/source_must_be_blocked.json.fixture")).unwrap()
}

fn lower(row: &serde_json::Value) -> compiler::cards::CardDefinition {
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    let (compiled, loss) = compiler::parse_loss::capture(|| {
        compiler::CompilerFacade::new().compile_definition(
            compiler::CardDefinitionBuilder::new(compiler::CardId::new(), name), text,
            compiler::CompilePolicy { allow_unsupported: false })
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    compiled.unwrap_or_else(|error| panic!("{name}: {error}")).definition
}

fn required(effect: &compiler::effect::Effect) {
    let effect = effect.downcast_ref::<CantEffect>().expect("typed must-be-blocked restriction");
    assert_eq!(effect.restriction, Restriction::MustBeBlocked(ObjectFilter::source()));
    assert_eq!(effect.duration, Until::EndOfTurn);
}

#[test]
fn frozen_metadata_and_every_sibling_survive_strict_whole_body_lowering() {
    let rows = rows();
    assert_eq!(rows.len(), 3);
    for (name, oracle_id, cost, pt, abilities) in [
        ("Anzrag, the Quake-Mole", "4adcd967-9ff7-4940-b8b9-0c4215bbcb75", "{2}{R}{G}", ("8", "4"), 2),
        ("Glorfindel, Dauntless Rescuer", "93842030-2017-4233-a7ac-7112361c019f", "{2}{G}", ("3", "2"), 1),
        ("Loathsome Catoblepas", "c6f68e4b-af2b-43d5-8052-104defe7f3ec", "{5}{B}", ("3", "3"), 2),
    ] {
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        assert_eq!(row["oracle_id"], oracle_id);
        assert_eq!(row["mana_cost"], cost);
        assert_eq!(row["power"], pt.0);
        assert_eq!(row["toughness"], pt.1);
        let definition = lower(row);
        assert_eq!(definition.abilities.len(), abilities, "{name}: no sibling may disappear");
        let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered), _ => None,
        }).expect("every frozen body has a sibling trigger");
        assert!(!triggered.effects.all_effects().is_empty());
        if name == "Glorfindel, Dauntless Rescuer" {
            let modal = triggered.effects.all_effects().into_iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>()).unwrap();
            assert_eq!(modal.min, Value::Fixed(1));
            assert_eq!(modal.max, Value::Fixed(1));
            assert_eq!(modal.modes.len(), 2);
            assert_eq!(modal.common_prefix_effects.len(), 1, "pump occurs once regardless of mode");
            let pump = modal.common_prefix_effects[0]
                .downcast_ref::<compiler::effects::ApplyContinuousEffect>().unwrap();
            assert_eq!(pump.until, Until::EndOfTurn);
            assert_eq!(pump.runtime_modifications, vec![
                compiler::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::Fixed(1), toughness: Value::Fixed(1),
                },
            ]);
            assert!(matches!(pump.target_spec.as_ref().map(|spec| spec.base()),
                Some(compiler::target::ChooseSpec::Source)));
            assert_eq!(modal.modes[0].effects.len(), 1);
            required(&modal.modes[0].effects[0]);
            assert!(!modal.modes[1].effects.is_empty(), "the maximum-one-blocker sibling is mandatory");
        } else {
            let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated), _ => None,
            }).unwrap();
            assert!(activated.choices.is_empty(), "source requirements do not announce a target");
            assert_eq!(activated.mana_cost.costs().len(), 1, "no invented tap or sacrifice cost");
            let cost = activated.mana_cost.mana_cost().unwrap();
            let expected = if name == "Anzrag, the Quake-Mole" { 7 } else { 3 };
            assert_eq!(cost.mana_value(), expected);
            use compiler::mana::{ManaCost, ManaSymbol};
            let expected_pips = if name == "Anzrag, the Quake-Mole" {
                vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Red],
                    vec![ManaSymbol::Red], vec![ManaSymbol::Green], vec![ManaSymbol::Green]]
            } else {
                vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Green]]
            };
            assert_eq!(cost, &ManaCost::from_pips(expected_pips));
            assert_eq!(activated.effects.all_effects().len(), 1);
            required(activated.effects.all_effects()[0]);
            if name == "Anzrag, the Quake-Mole" {
                let phases = triggered.effects.all_effects().into_iter()
                    .find_map(|effect| effect.downcast_ref::<AdditionalPhasesEffect>()).unwrap();
                assert_eq!(phases.phases, vec![AdditionalPhase::Combat]);
                assert!(!phases.after_main_phase, "after this phase is not restricted to main phases");
                assert_eq!(triggered.effects.all_effects().len(), 2, "untap and additional combat both remain");
            }
        }
    }
}

#[test]
fn unsupported_sibling_and_incomplete_requirement_are_not_full_card_credit() {
    for row in rows() {
        for extra in [
            "\nAt the beginning of your upkeep, frobnicate this creature.",
            "\n{1}: This creature must be blocked next turn if able.",
            "\n{1}: This {R} creature must be blocked this turn if able.",
        ] {
            let name = row["name"].as_str().unwrap();
            let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}{extra}",
                row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
                row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
                row["oracle_text"].as_str().unwrap());
            assert!(compiler::CompilerFacade::new().compile_definition(
                compiler::CardDefinitionBuilder::new(compiler::CardId::new(), name), text,
                compiler::CompilePolicy { allow_unsupported: false }).is_err(), "{name}: {extra}");
        }
    }
}

#[test]
fn punctuation_before_the_source_requirement_is_not_erased_in_a_complete_card() {
    let rows = rows();
    let row = rows.iter().find(|row| row["name"] == "Loathsome Catoblepas").unwrap();
    // Keep the death trigger intact. The only difference from the valid full
    // frozen body is raw punctuation before the activation's requirement.
    let original = row["oracle_text"].as_str().unwrap();
    assert!(original.contains("This creature must be blocked"));
    for punctuation in [",", ".", ";", "\""] {
        let malformed = original.replace("This creature must be blocked",
            &format!("This creature{punctuation} must be blocked"));
        let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{malformed}",
            row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
            row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap());
        assert!(compiler::CompilerFacade::new().compile_definition(
            compiler::CardDefinitionBuilder::new(compiler::CardId::new(), "Loathsome Catoblepas"),
            &text, compiler::CompilePolicy { allow_unsupported: false }).is_err(),
            "strict lowering must reject source punctuation {punctuation:?}");
        assert!(ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Loathsome Catoblepas", &text, false).is_err(),
            "direct route must reject source punctuation {punctuation:?}");
        assert!(ironsmith_compiler_runtime::compile_to_artifact(
            "Loathsome Catoblepas", &text, false).is_err(),
            "artifact route must reject source punctuation {punctuation:?}");
    }
}
