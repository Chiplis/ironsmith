use crate::cards::builders::{CardTextError, GrantedAbilityAst, StaticAbilityAst};
use crate::costs::Cost;
use crate::model::{CompilerAbilityCore, CompilerAbilityKindCore, CompilerStaticAbilityCore};
use crate::static_abilities::StaticAbility;

pub type DerivedAlternativeCast = ironsmith_core::DerivedAlternativeCast<Cost>;
pub type Grantable = ironsmith_core::Grantable<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCastCondition,
>;
pub type GrantSpec = ironsmith_core::GrantSpec<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCastCondition,
>;
pub use ironsmith_core::{GrantDuration, GrantUsageLimit, SourceExiledGrantSurface};

/// Preserve a parsed granted ability in the compiler-owned generic ability
/// algebra. Runtime effects, triggers, and costs are materialized only by the
/// lowering module.
pub fn compiler_granted_ability_ast_to_object_ability(
    ability: &GrantedAbilityAst,
) -> Result<CompilerAbilityCore, CardTextError> {
    let static_ability = |ability| Ok(CompilerAbilityCore::static_ability(ability));
    match ability {
        GrantedAbilityAst::Bundle(_) => Err(CardTextError::InvariantViolation(
            "a composite granted ability must lower through the complete list owner".into(),
        )),
        GrantedAbilityAst::KeywordAction(action) => static_ability(
            crate::keyword_abilities::static_ability_for_keyword_action((**action).clone())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "keyword grant requires executable lowering: {action:?}"
                    ))
                })?,
        ),
        GrantedAbilityAst::StaticAbility(ability) => match ability.as_ref() {
            StaticAbilityAst::Static(ability) => static_ability(ability.clone()),
            other => Err(CardTextError::ParseError(format!(
                "nested static grant requires structural lowering: {other:?}"
            ))),
        },
        GrantedAbilityAst::MustAttack => static_ability(CompilerStaticAbilityCore::must_attack()),
        GrantedAbilityAst::MustBlock => static_ability(CompilerStaticAbilityCore::must_block()),
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            static_ability(CompilerStaticAbilityCore::can_attack_as_though_no_defender())
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => static_ability(
            CompilerStaticAbilityCore::can_block_additional_creature_each_combat(*additional),
        ),
        GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
            let mut compiler_ability = (*ability.ability).clone();
            if let Some(effects) = &ability.effects_ast {
                match &mut compiler_ability.kind {
                    CompilerAbilityKindCore::Triggered(triggered) => {
                        triggered.effects =
                            ironsmith_core::ResolutionProgram::from_effects(effects.clone());
                    }
                    CompilerAbilityKindCore::Activated(activated) => {
                        activated.effects =
                            ironsmith_core::ResolutionProgram::from_effects(effects.clone());
                    }
                    _ => {}
                }
            }
            if let Some(trigger) = &ability.trigger_spec
                && let CompilerAbilityKindCore::Triggered(triggered) = &mut compiler_ability.kind
            {
                triggered.trigger = (**trigger).clone();
            }
            Ok(compiler_ability)
        }
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot be stored as an independent object ability".to_string(),
        )),
        GrantedAbilityAst::StaticAbilityFamily(_) => Err(CardTextError::InvariantViolation(
            "an ability family can only be lost, not stored as an object ability".to_string(),
        )),
    }
}
