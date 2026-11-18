use crate::cards::builders::{CardTextError, GrantedAbilityAst, KeywordAction};
use crate::model::CompilerAbilityCore;
pub use ironsmith_compiler_semantic::keyword_abilities::static_ability_for_keyword_action;

/// Preserve a parsed granted ability in the compiler-owned generic ability
/// algebra. Runtime effects, triggers, and costs are materialized only by the
/// lowering module.
pub fn compiler_granted_ability_ast_to_object_ability(
    ability: &GrantedAbilityAst,
) -> Result<CompilerAbilityCore, CardTextError> {
    match ability {
        GrantedAbilityAst::KeywordAction(action) => {
            if let Some(ability) =
                ironsmith_compiler_semantic::keyword_abilities::attack_keyword_granted_ability(
                    action,
                )
            {
                return Ok(ability);
            }
            if let Some(ability) = static_ability_for_keyword_action((**action).clone()) {
                return Ok(CompilerAbilityCore::static_ability(ability));
            }
            // Activated keywords such as Crew grant a real activated ability
            // ("except it's a Vehicle artifact with crew 3", Imposter Mech).
            if let Some(parsed) =
                crate::keyword_static::parse_nonstatic_keyword_action_as_object_ability(
                    (**action).clone(),
                )
            {
                return Ok(*parsed.ability);
            }
            Err(CardTextError::ParseError(format!(
                "keyword grant requires executable lowering: {action:?}"
            )))
        }
        GrantedAbilityAst::Bundle(_) => Err(CardTextError::InvariantViolation(
            "a composite granted ability requires its complete list owner".into(),
        )),
        other => {
            ironsmith_compiler_semantic::grant::compiler_granted_ability_ast_to_object_ability(
                other,
            )
        }
    }
}

pub fn compiler_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<CompilerAbilityCore>, CardTextError> {
    let mut expanded = Vec::new();
    for ability in abilities {
        if let GrantedAbilityAst::Bundle(members) = ability {
            expanded.extend(compiler_granted_abilities_ast_to_object_abilities(members)?);
            continue;
        }
        if let GrantedAbilityAst::KeywordAction(action) = ability
            && let KeywordAction::Vanishing(amount) = action.as_ref()
        {
            expanded.extend(
                ironsmith_compiler_semantic::keyword_abilities::vanishing_granted_abilities(
                    *amount,
                ),
            );
        } else {
            expanded.push(compiler_granted_ability_ast_to_object_ability(ability)?);
        }
    }
    Ok(expanded)
}
