use super::super::super::util::tokenize_line;
use super::*;
use crate::CardId;
use crate::cards::builders::CardDefinitionBuilder;

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
                    SubjectVerbActionAst::PumpAll {
                        filter,
                        power,
                        toughness,
                        duration,
                        ..
                    },
                ..
            }) => Some((filter, power, toughness, duration)),
            EffectAst::Sequence { effects }
            | EffectAst::Coordinated {
                effects,
                leading_duration: _,
            } => effects.iter().find_map(find_pump),
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
                    SubjectVerbActionAst::GrantAbilitiesAll {
                        filter,
                        abilities,
                        duration,
                        ..
                    },
                ..
            }) => Some((filter, abilities, duration)),
            EffectAst::Sequence { effects }
            | EffectAst::Coordinated {
                effects,
                leading_duration: _,
            } => effects.iter().find_map(find_grant),
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
        GrantedAbilityAst::KeywordAction(KeywordAction::Vigilance) => true,
        GrantedAbilityAst::StaticAbility(ability) => {
            ability.id() == StaticAbilityId::Vigilance
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
    eprintln!("{}", trace.render());
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

    let citizen_effects: Vec<_> = effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .filter_map(continuous_effect)
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
