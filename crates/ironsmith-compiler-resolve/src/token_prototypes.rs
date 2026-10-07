//! Lexical token-blueprint binding, independent of dynamic object memory.
//!
//! An "otherwise" arm and a later die-result row can name a blueprint in an
//! arm that will never execute. Resolve that definition before branch-local
//! object references and before lowering into complete native create effects.

use crate::cards::builders::{CardTextError, EffectAst, SubjectVerbActionAst, TokenActionAst};
use crate::model::control_flow::ControlFlowNodeAst;
use crate::model::token_definition::TokenDefinitionSpec;
use crate::model::visit::try_for_each_nested_effects_mut;

pub fn resolve_token_prototypes(effects: &mut [EffectAst]) -> Result<(), CardTextError> {
    bind_sequence(effects, &mut None)
}

fn bind_sequence(
    effects: &mut [EffectAst],
    previous: &mut Option<TokenActionAst>,
) -> Result<(), CardTextError> {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject) = effect
            && let SubjectVerbActionAst::Tokens(action) = &mut subject.action
            && let TokenActionAst::CreateTokenWithMods { definition, .. } = action
        {
            if matches!(definition, TokenDefinitionSpec::PrototypeReference(_)) {
                let Some(prototype) = previous.as_ref() else {
                    return Err(CardTextError::ParseError(
                        "token prototype reference has no preceding authored definition in this ability".into(),
                    ));
                };
                bind_action(action, prototype);
            } else {
                *previous = Some(action.clone());
            }
        }
        match effect {
            // The original instruction is authored before the replacement,
            // although the executable AST stores the replacement branch first.
            EffectAst::SelfReplacement { if_true, if_false, .. } => {
                bind_sequence(if_false, previous)?;
                bind_sequence(if_true, previous)?;
            }
            EffectAst::ControlFlow(control) => {
                let original = match &control.node {
                    ControlFlowNodeAst::Replacement(replacement) => replacement.original_program,
                    _ => None,
                };
                if let Some(original) = original {
                    bind_sequence(&mut control.programs[original].effects, previous)?;
                }
                for (index, program) in control.programs.iter_mut().enumerate() {
                    if Some(index) == original {
                        continue;
                    }
                    if program.kind == crate::model::control_flow::NestedProgramKindAst::NestedAbility {
                        bind_sequence(&mut program.effects, &mut None)?;
                    } else {
                        bind_sequence(&mut program.effects, previous)?;
                    }
                }
            }
            _ => try_for_each_nested_effects_mut(effect, true, |nested| {
                bind_sequence(nested, previous)
            })?,
        }
    }
    Ok(())
}

fn bind_action(action: &mut TokenActionAst, prototype: &TokenActionAst) {
    let TokenActionAst::CreateTokenWithMods {
        count, player, actor_surface_explicit, tapped, attacking,
        attached_to, dynamic_power_toughness, attack_target_player, combat_entry,
        exile_at_end_of_combat, sacrifice_at_end_of_combat, sacrifice_at_next_end_step,
        exile_at_next_end_step, next_end_step_player, granted_abilities, ability_presentation, ..
    } = action else { unreachable!("only token definition references are bound") };
    let mut bound = prototype.clone();
    let TokenActionAst::CreateTokenWithMods {
        count: bound_count, player: bound_player, actor_surface_explicit: bound_actor,
        tapped: bound_tapped, attacking: bound_attacking, attached_to: bound_attachment,
        dynamic_power_toughness: bound_pt, attack_target_player: bound_attack_player,
        combat_entry: bound_combat, exile_at_end_of_combat: bound_combat_exile,
        sacrifice_at_end_of_combat: bound_combat_sacrifice,
        sacrifice_at_next_end_step: bound_end_sacrifice, exile_at_next_end_step: bound_end_exile,
        next_end_step_player: bound_end_player, granted_abilities: bound_grants,
        ability_presentation: bound_presentation, ..
    } = &mut bound else { unreachable!("only authored token definitions are remembered") };
    *bound_count = count.clone();
    *bound_player = *player;
    *bound_actor = *actor_surface_explicit;
    *bound_tapped |= *tapped;
    *bound_attacking |= *attacking;
    if attached_to.is_some() { *bound_attachment = attached_to.clone(); }
    if dynamic_power_toughness.is_some() { *bound_pt = dynamic_power_toughness.clone(); }
    if attack_target_player.is_some() { *bound_attack_player = *attack_target_player; }
    if *combat_entry != Default::default() { *bound_combat = combat_entry.clone(); }
    *bound_combat_exile |= *exile_at_end_of_combat;
    *bound_combat_sacrifice |= *sacrifice_at_end_of_combat;
    *bound_end_sacrifice |= *sacrifice_at_next_end_step;
    *bound_end_exile |= *exile_at_next_end_step;
    if *next_end_step_player != crate::PlayerFilter::Any {
        *bound_end_player = next_end_step_player.clone();
    }
    bound_grants.extend(granted_abilities.iter().cloned());
    if ability_presentation.is_some() { *bound_presentation = *ability_presentation; }
    *action = bound;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{ConditionalEffectAst, PlayerAst, PredicateAst, SubjectVerbRoleAst};
    use crate::model::token_definition::{BuiltinTokenShape, TokenPrototypeReference};
    use crate::{ObjectFilter, PlayerFilter, Value};

    fn create(definition: TokenDefinitionSpec, count: i32, tapped: bool) -> EffectAst {
        EffectAst::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                name: "Authored token".into(), definition, count: Value::Fixed(count),
                dynamic_power_toughness: None, player: PlayerAst::Implicit,
                actor_surface_explicit: false, attached_to: None, tapped, attacking: false,
                attack_target_player: None, combat_entry: Default::default(),
                exile_at_end_of_combat: false, sacrifice_at_end_of_combat: false,
                sacrifice_at_next_end_step: true, exile_at_next_end_step: false,
                next_end_step_player: PlayerFilter::Any, granted_abilities: Vec::new(),
                ability_presentation: None,
            }))
    }
    fn reference(count: i32) -> EffectAst {
        create(TokenDefinitionSpec::PrototypeReference(TokenPrototypeReference::PreviousDefinition), count, false)
    }
    fn check(effect: &EffectAst, builtin: BuiltinTokenShape, expected_count: i32) {
        let EffectAst::SubjectVerb(subject) = effect else { panic!("{effect:?}") };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            definition, count, tapped, sacrifice_at_next_end_step, ..
        }) = &subject.action else { panic!("expected token") };
        assert_eq!(*definition, TokenDefinitionSpec::Builtin(builtin));
        assert_eq!(*count, Value::Fixed(expected_count));
        assert!(*tapped);
        assert!(*sacrifice_at_next_end_step);
    }

    #[test]
    fn replacement_and_otherwise_bind_unexecuted_blueprints_in_authored_order() {
        let original = create(TokenDefinitionSpec::Builtin(BuiltinTokenShape::Treasure), 1, true);
        for mut effects in [
            vec![EffectAst::SelfReplacement {
                predicate: PredicateAst::YouControl(ObjectFilter::creature()),
                if_false: vec![original.clone()], if_true: vec![reference(3)],
                attach_to_previous_ability: false,
            }],
            vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::YouControl(ObjectFilter::creature()),
                if_true: vec![original.clone()], if_false: vec![reference(3)],
            })],
        ] {
            resolve_token_prototypes(&mut effects).unwrap();
            let repeated = match &effects[0] {
                EffectAst::SelfReplacement { if_true, .. } => &if_true[0],
                EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_false, .. }) => &if_false[0],
                _ => unreachable!(),
            };
            check(repeated, BuiltinTokenShape::Treasure, 3);
        }
    }

    #[test]
    fn references_are_lexical_and_cannot_import_another_abilitys_definition() {
        let mut effects = vec![
            create(TokenDefinitionSpec::Builtin(BuiltinTokenShape::Treasure), 1, true),
            reference(2),
            create(TokenDefinitionSpec::Builtin(BuiltinTokenShape::EldraziSpawn), 1, true),
            reference(3),
        ];
        resolve_token_prototypes(&mut effects).unwrap();
        check(&effects[1], BuiltinTokenShape::Treasure, 2);
        check(&effects[3], BuiltinTokenShape::EldraziSpawn, 3);
        assert!(resolve_token_prototypes(&mut [reference(1)]).is_err());
    }
}
