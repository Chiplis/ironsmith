//! Abilities a keyword stands for.
//!
//! A keyword names an ability; these build the ability it names. Both the
//! recognizer (which reports the keyword as a fact) and lowering (which
//! materializes it) need the same construction, so it lives here.

use ironsmith_core::TotalCost;

use crate::cards::builders::{
    EffectAst, PlayerAst, PredicateAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
    TriggerSpec, KeywordActionAst,
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
                    SubjectVerbActionAst::KeywordActions(KeywordActionAst::CumulativeUpkeep { cost: total_cost }),
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
