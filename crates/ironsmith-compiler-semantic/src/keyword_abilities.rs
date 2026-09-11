//! Abilities a keyword stands for.
//!
//! A keyword names an ability; these build the ability it names. Both the
//! recognizer (which reports the keyword as a fact) and lowering (which
//! materializes it) need the same construction, so it lives here.

use ironsmith_core::TotalCost;

use crate::cards::builders::{
    EffectAst, KeywordActionAst, PlayerAst, PredicateAst, SubjectVerbActionAst, SubjectVerbRoleAst,
    TargetAst, TriggerSpec,
};
use crate::effect::Value;
use crate::filter::PlayerFilter;
use crate::model::CompilerCost;
use crate::model::ParsedAbility;
use crate::model::reference_state::ReferenceImports;
use crate::model::{
    CompilerAbilityCore as Ability, CompilerAbilityKindCore as AbilityKind,
    CompilerTriggeredAbilityCore as TriggeredAbility,
};
use crate::object::CounterType;
use crate::zone::Zone;

pub fn cumulative_upkeep_granted_ability(total_cost: TotalCost<CompilerCost>) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: TriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            effects: ironsmith_core::ResolutionProgram::from_effects(vec![
                EffectAst::subject_verb_put_counters(
                    CounterType::Age,
                    Value::Fixed(1),
                    TargetAst::Source(None),
                    None,
                    false,
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::You,
                    SubjectVerbActionAst::KeywordActions(KeywordActionAst::CumulativeUpkeep {
                        cost: total_cost,
                    }),
                ),
            ]),
            choices: vec![],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

/// Assemble a recognized triggered ability.
///
/// `intervening_if` is the predicate the recognizer read, not a resolved
/// condition — recognizers record what the text says and let the resolver bind
/// it.
pub fn assemble_parsed_triggered_ability(
    trigger: TriggerSpec,
    effects_ast: Vec<EffectAst>,
    functional_zones: Vec<Zone>,
    intervening_if: Option<PredicateAst>,
    presentation_label: Option<&crate::ability::PresentationLabel>,
    reference_imports: impl Into<ReferenceImports>,
) -> ParsedAbility {
    let reference_imports = reference_imports.into();
    ParsedAbility {
        ability: crate::model::CompilerAbilityCore {
            kind: crate::model::CompilerAbilityKindCore::Triggered(
                crate::model::CompilerTriggeredAbilityCore {
                    trigger: trigger.clone(),
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: Vec::new(),
                    intervening_if,
                    presentation_label: presentation_label.cloned(),
                },
            ),
            functional_zones,
        }
        .into(),
        effects_ast: Some(effects_ast),
        trigger_spec: Some(Box::new(trigger)),
        reference_imports,
    }
}

/// The abilities represented by vanishing, including bare vanishing at zero.
pub fn vanishing_granted_abilities(amount: u32) -> Vec<Ability> {
    let mut abilities = Vec::new();
    if amount > 0 {
        abilities.push(Ability::static_ability(
            crate::model::CompilerStaticAbilityCore::enters_with_counters_value(
                CounterType::Time,
                Value::Fixed(amount as i32),
            ),
        ));
    }
    let source = crate::filter::ObjectFilter::source();
    for (trigger, effect) in [
        (
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            EffectAst::subject_verb_remove_counters_all(
                Value::Fixed(1),
                source.clone(),
                Some(CounterType::Time),
                false,
            ),
        ),
        (
            TriggerSpec::CounterRemovedFrom {
                filter: source.clone(),
                counter_type: Some(CounterType::Time),
                last: true,
                one_or_more: false,
                caused_by_source: false,
            },
            EffectAst::subject_verb_sacrifice(
                PlayerAst::You,
                source,
                1,
                Some(TargetAst::Source(None)),
            ),
        ),
    ] {
        let intervening_if = matches!(trigger, TriggerSpec::BeginningOfUpkeep(_)).then(|| {
            PredicateAst::Source(crate::model::SourcePredicateAst::SourceHasCounterAtLeast {
                counter_type: CounterType::Time,
                count: 1,
                surface: Default::default(),
            })
        });
        abilities.push(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects: ironsmith_core::ResolutionProgram::from_effects(vec![effect]),
                choices: vec![],
                intervening_if,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    }
    abilities
}
