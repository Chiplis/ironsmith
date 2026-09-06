use crate::cards::builders::StatChangeActionAst;
use crate::cards::builders::GrantActionAst;
use super::super::super::util::tokenize_line;
use super::*;
use crate::CardId;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

#[test]
fn quoted_source_relative_restriction_remains_a_temporary_static_grant() {
    for text in [
        "Until end of turn, this creature gains \"Creatures dealt damage by this creature this turn can't be regenerated this turn.\"",
        "Until end of turn, this creature gains Creatures dealt damage by this creature this turn can't be regenerated this turn.",
    ] {
        let tokens = tokenize_line(text, 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("source-relative restriction grant should parse")
            .expect("source-relative restriction grant should produce effects");

        let ast_debug = format!("{effects:#?}");
        assert!(ast_debug.contains("GrantAbilitiesToTarget"), "{ast_debug}");
        assert!(ast_debug.contains("duration: EndOfTurn"), "{ast_debug}");
        assert!(
            ast_debug.contains("dealt_damage_by_source_this_turn: Some(\n")
                && ast_debug.contains("ThisCreature"),
            "{ast_debug}"
        );

        let compiled = compile_statement_effects(&effects)
            .expect("source-relative restriction grant should lower");
        let compiled_debug = format!("{compiled:#?}");
        assert!(
            compiled_debug.contains("ApplyContinuousEffect"),
            "{compiled_debug}"
        );
        assert!(compiled_debug.contains("AddAbility"), "{compiled_debug}");
        assert!(
            compiled_debug.contains("dealt_damage_by_source_this_turn: Some(\n")
                && compiled_debug.contains("ThisCreature"),
            "{compiled_debug}"
        );
    }
}

#[test]
fn activated_quoted_source_relative_restriction_uses_the_grant_pipeline() {
    let (definition, trace) = crate::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::new(), "Source Relative Restriction Probe")
            .card_types(vec![crate::types::CardType::Creature])
            .parse_text(
                "{B}: Until end of turn, this creature gains \"Creatures dealt damage by this creature this turn can't be regenerated this turn.\"",
            )
    });
    let definition = definition.expect("activated source-relative restriction grant should parse");
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("ApplyContinuousEffect"),
        "{trace:#?}\n{debug}"
    );
    assert!(debug.contains("AddAbility"), "{trace:#?}\n{debug}");
    assert!(
        debug.contains("dealt_damage_by_source_this_turn: Some(\n")
            && debug.contains("ThisCreature"),
        "{trace:#?}\n{debug}"
    );
}

#[test]
fn additional_pump_and_ability_grant_are_both_present_in_semantic_ast() {
    let tokens = tokenize_line(
        "Soldiers you control get an additional +1/+1 and gain vigilance until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("additional pump plus ability grant should parse")
        .expect("additional pump plus ability grant should produce effects");

    fn find_pump(effect: &EffectAst) -> Option<(&ObjectFilter, &Value, &Value, &Until)> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                        filter,
                        power,
                        toughness,
                        duration,
                        ..
                    }),
                ..
            }) => Some((filter, power, toughness, duration)),
            EffectAst::Sequence { effects }
            | EffectAst::Coordinated {
                effects,
                leading_duration: _,
                result_conjunction: _,
            } => effects.iter().find_map(find_pump),
            EffectAst::Coordination(coordination) => coordination.effects().find_map(find_pump),
            EffectAst::ControlFlow(control) => control
                .programs
                .iter()
                .find_map(|program| program.effects.iter().find_map(find_pump)),
            _ => None,
        }
    }
    let pump = effects.iter().find_map(find_pump);
    let (pump_filter, power, toughness, pump_duration) =
        pump.expect("semantic AST should contain the additional +1/+1 pump");
    assert_eq!(pump_filter.controller, Some(PlayerFilter::You));
    assert!(
        pump_filter
            .subtypes
            .contains(&crate::types::Subtype::Soldier)
    );
    assert_eq!((power, toughness), (&Value::Fixed(1), &Value::Fixed(1)));
    assert_eq!(pump_duration, &Until::EndOfTurn);

    fn find_grant(effect: &EffectAst) -> Option<(&ObjectFilter, &Vec<GrantedAbilityAst>, &Until)> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                        filter,
                        abilities,
                        duration,
                        ..
                    }),
                ..
            }) => Some((filter, abilities, duration)),
            EffectAst::Sequence { effects }
            | EffectAst::Coordinated {
                effects,
                leading_duration: _,
                result_conjunction: _,
            } => effects.iter().find_map(find_grant),
            EffectAst::Coordination(coordination) => coordination.effects().find_map(find_grant),
            EffectAst::ControlFlow(control) => control
                .programs
                .iter()
                .find_map(|program| program.effects.iter().find_map(find_grant)),
            _ => None,
        }
    }
    let grant = effects.iter().find_map(find_grant);
    let (grant_filter, abilities, grant_duration) =
        grant.expect("semantic AST should contain the vigilance grant");
    assert_eq!(grant_filter.controller, Some(PlayerFilter::You));
    assert!(
        grant_filter
            .subtypes
            .contains(&crate::types::Subtype::Soldier)
    );
    assert_eq!(grant_duration, &Until::EndOfTurn);
    assert!(abilities.iter().any(|ability| match ability {
        GrantedAbilityAst::KeywordAction(action) => {
            matches!(action.as_ref(), KeywordAction::Vigilance)
        }
        GrantedAbilityAst::StaticAbility(ability) => {
            matches!(
                ability.as_ref(),
                crate::cards::builders::StaticAbilityAst::Static(ability)
                    if ability.id() == StaticAbilityId::Vigilance
            )
        }
        _ => false,
    }));
}

#[test]
fn take_to_the_streets_strictly_compiles_both_citizen_modifications() {
    let (definition, trace) = crate::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::from_raw(1), "Take to the Streets")
            .card_types(vec![crate::types::CardType::Sorcery])
            .parse_text(
                "Creatures you control get +2/+2 until end of turn. Citizens you control get an additional +1/+1 and gain vigilance until end of turn.",
            )
    });
    let _ = trace;
    let definition = definition.expect("Take to the Streets should parse strictly");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Take to the Streets should have a spell effect");

    fn continuous_effect(
        effect: &crate::effect::Effect,
    ) -> Option<&crate::effects::ApplyContinuousEffect> {
        effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| continuous_effect(&tagged.effect))
            })
    }

    let flattened_effects = effects.flattened_default_effects();
    fn collect_continuous_effects<'a>(
        effect: &'a crate::effect::Effect,
        output: &mut Vec<&'a crate::effects::ApplyContinuousEffect>,
    ) {
        if let Some(apply) = continuous_effect(effect) {
            output.push(apply);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            for child in &sequence.effects {
                collect_continuous_effects(child, output);
            }
        }
    }

    let mut continuous_effects = Vec::new();
    for effect in flattened_effects {
        collect_continuous_effects(effect, &mut continuous_effects);
    }
    let citizen_effects: Vec<_> = continuous_effects
        .into_iter()
        .filter(|apply| {
            matches!(
                &apply.target,
                crate::continuous::EffectTarget::Filter(filter)
                    if filter.controller == Some(PlayerFilter::You)
                        && filter.subtypes.contains(&crate::types::Subtype::Citizen)
            )
        })
        .collect();
    assert_eq!(citizen_effects.len(), 2, "{citizen_effects:#?}");
    assert!(
        citizen_effects.iter().any(|apply| {
            apply.runtime_modifications.iter().any(|modification| {
                matches!(
                    modification,
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: Value::Fixed(1),
                        toughness: Value::Fixed(1),
                    }
                )
            })
        }),
        "missing Citizen +1/+1 effect: {citizen_effects:#?}"
    );
    assert!(
        citizen_effects.iter().any(|apply| {
            apply.modification.as_ref().is_some_and(|modification| {
                matches!(
                    modification,
                    crate::continuous::Modification::AddAbility(ability)
                        if ability.id() == StaticAbilityId::Vigilance
                )
            })
        }),
        "missing Citizen vigilance effect: {citizen_effects:#?}"
    );
    assert!(
        citizen_effects
            .iter()
            .all(|apply| apply.until == Until::EndOfTurn)
    );
}
