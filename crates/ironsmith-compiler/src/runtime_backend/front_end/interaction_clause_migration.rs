//! Finite PR-27 adapter from interaction leaves to typed common clauses.

use crate::effect::Until;
use crate::model::ast::{EffectAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActionAst, ClauseObjectAst, ClausePolarityAst, ClauseSubjectAst, ClauseVerbAst,
    CompilerClauseAst,
};
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::interaction_clauses::{
    CompilerCharacteristicClauseAst, CompilerCharacteristicOperationAst, CompilerCombatClauseAst,
    CompilerCombatOperationAst, CompilerCombatRoleAst, CompilerCounterAmountAst,
    CompilerCounterClauseAst, CompilerCounterOperationAst, CompilerDamageClauseAst,
    CompilerDamageDivisionAst, CompilerInteractionClauseAst, CompilerModificationModeAst,
    CompilerPreventionClauseAst, CompilerPreventionKindAst,
};
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::static_abilities::ContinuousLayerAst;
use crate::model::symbols::{Cardinality, ObjectDomain, ReferenceRole, SymbolResolutionError};
use crate::model::visit::for_each_nested_effect_vec_mut;
use crate::runtime_backend::front_end::semantic_migration_context::SemanticMigrationContext;

struct InteractionMigration<'migration, 'symbols> {
    context: &'migration mut SemanticMigrationContext<'symbols>,
}

impl<'migration, 'symbols> InteractionMigration<'migration, 'symbols> {
    fn migrate_effects(
        &mut self,
        effects: &mut Vec<EffectAst>,
    ) -> Result<(), SymbolResolutionError> {
        for effect in effects {
            self.migrate_effect(effect)?;
        }
        Ok(())
    }

    fn migrate_effect(&mut self, effect: &mut EffectAst) -> Result<(), SymbolResolutionError> {
        if let Some(clause) = self.interaction_clause(effect)? {
            *effect = EffectAst::Clause(clause);
            return Ok(());
        }
        let mut result = Ok(());
        for_each_nested_effect_vec_mut(effect, true, |nested| {
            if result.is_ok() {
                result = self.migrate_effects(nested);
            }
        });
        result
    }

    fn interaction_clause(
        &mut self,
        effect: &EffectAst,
    ) -> Result<Option<CompilerClauseAst>, SymbolResolutionError> {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return Ok(None);
        };
        let actor = super::library_clause_migration::compiler_actor(subject_verb.subject.player);
        let (verb, object, duration, interaction) = match &subject_verb.action {
            SubjectVerbActionAst::DealDamage {
                amount,
                target,
                unpreventable,
            } => {
                let recipients = self.context.target_operand(target)?;
                let result = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::DealDamage,
                    Some(recipients.clone()),
                    None,
                    CompilerInteractionClauseAst::Damage(CompilerDamageClauseAst {
                        source: CompilerObjectOperandAst::Source,
                        recipients,
                        amount: CompilerValueAst::Dynamic(amount.clone()),
                        division: CompilerDamageDivisionAst::None,
                        chooser: None,
                        combat_damage: false,
                        unpreventable: *unpreventable,
                        result,
                    }),
                )
            }
            SubjectVerbActionAst::DealDamageEach { amount, filter } => {
                let recipients =
                    CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter.clone()));
                let result = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::DealDamage,
                    Some(recipients.clone()),
                    None,
                    CompilerInteractionClauseAst::Damage(CompilerDamageClauseAst {
                        source: CompilerObjectOperandAst::Source,
                        recipients,
                        amount: CompilerValueAst::Dynamic(amount.clone()),
                        division: CompilerDamageDivisionAst::None,
                        chooser: None,
                        combat_damage: false,
                        unpreventable: false,
                        result,
                    }),
                )
            }
            SubjectVerbActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable,
            } => {
                let source = self.context.target_operand(source)?;
                let recipients = self.context.target_operand(target)?;
                let result = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::DealDamage,
                    Some(recipients.clone()),
                    None,
                    CompilerInteractionClauseAst::Damage(CompilerDamageClauseAst {
                        source,
                        recipients,
                        amount: CompilerValueAst::Dynamic(amount.clone()),
                        division: CompilerDamageDivisionAst::None,
                        chooser: None,
                        combat_damage: false,
                        unpreventable: *unpreventable,
                        result,
                    }),
                )
            }
            SubjectVerbActionAst::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
                distribution,
            } => {
                let source = self.context.target_operand(source)?;
                let recipients = self.context.target_operand(target)?;
                let result = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::DealDamage,
                    Some(recipients.clone()),
                    None,
                    CompilerInteractionClauseAst::Damage(CompilerDamageClauseAst {
                        source,
                        recipients,
                        amount: CompilerValueAst::Dynamic(amount.clone()),
                        division: match distribution {
                            ironsmith_core::DamageDistributionMode::Chosen => {
                                CompilerDamageDivisionAst::AsChosen
                            }
                            ironsmith_core::DamageDistributionMode::EvenRoundedDown => {
                                CompilerDamageDivisionAst::Evenly
                            }
                        },
                        chooser: Some(CompilerFilterAst::Player(chooser.clone())),
                        combat_damage: false,
                        unpreventable: false,
                        result,
                    }),
                )
            }
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            } => {
                let primary = self.context.target_operand(creature1)?;
                let opposing = self.context.target_operand(creature2)?;
                (
                    ClauseVerbAst::Fight,
                    Some(primary.clone()),
                    None,
                    CompilerInteractionClauseAst::Combat(CompilerCombatClauseAst {
                        operation: CompilerCombatOperationAst::Fight,
                        primary,
                        primary_role: CompilerCombatRoleAst::Fighter,
                        opposing: Some(opposing),
                        opposing_role: Some(CompilerCombatRoleAst::Fighter),
                        duration: None,
                    }),
                )
            }
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                follow_up_effects,
            } if !*source_of_your_choice
                && !*protect_you_and_permanents_you_control
                && follow_up_effects.is_empty() =>
            {
                let recipient = self.context.target_operand(target)?;
                let Some(duration) = compiler_duration(duration) else {
                    return Ok(None);
                };
                let shield = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::Prevent,
                    Some(recipient.clone()),
                    Some(duration.clone()),
                    CompilerInteractionClauseAst::Prevention(CompilerPreventionClauseAst {
                        kind: CompilerPreventionKindAst::Amount,
                        source: None,
                        recipient: Some(recipient),
                        amount: Some(CompilerValueAst::Dynamic(amount.clone())),
                        duration: Some(duration),
                        redirect_to: None,
                        shield,
                    }),
                )
            }
            SubjectVerbActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_target,
                ..
            } if !*source_of_your_choice => {
                let recipient = self.context.target_operand(target)?;
                let source = source_target
                    .as_ref()
                    .map(|source| self.context.target_operand(source))
                    .transpose()?;
                let Some(duration) = compiler_duration(duration) else {
                    return Ok(None);
                };
                let shield = self.effect_result(ReferenceRole::Affected)?;
                (
                    ClauseVerbAst::Prevent,
                    Some(recipient.clone()),
                    Some(duration.clone()),
                    CompilerInteractionClauseAst::Prevention(CompilerPreventionClauseAst {
                        kind: CompilerPreventionKindAst::All,
                        source,
                        recipient: Some(recipient),
                        amount: None,
                        duration: Some(duration),
                        redirect_to: None,
                        shield,
                    }),
                )
            }
            SubjectVerbActionAst::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            } => {
                let object = if let Some(target_count) = target_count {
                    self.context
                        .counted_target_operand(target, *target_count, None)?
                } else {
                    self.context.target_operand(target)?
                };
                self.counter_parts(
                    ClauseVerbAst::Add,
                    CompilerCounterOperationAst::Add,
                    Some(counter_type.clone()),
                    CompilerCounterAmountAst::Value(CompilerValueAst::Dynamic(count.clone())),
                    object,
                    None,
                    *distributed,
                )?
            }
            SubjectVerbActionAst::PutCountersAll {
                counter_type,
                count,
                filter,
            } => self.counter_parts(
                ClauseVerbAst::Add,
                CompilerCounterOperationAst::Add,
                Some(counter_type.clone()),
                CompilerCounterAmountAst::Value(CompilerValueAst::Dynamic(count.clone())),
                CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter.clone())),
                None,
                false,
            )?,
            SubjectVerbActionAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                distributed_across_all,
                ..
            } => {
                let object = self.context.target_operand(target)?;
                self.counter_parts(
                    ClauseVerbAst::Remove,
                    CompilerCounterOperationAst::Remove,
                    counter_type.clone(),
                    CompilerCounterAmountAst::Value(CompilerValueAst::Dynamic(amount.clone())),
                    object,
                    None,
                    *distributed_across_all,
                )?
            }
            SubjectVerbActionAst::MoveAllCounters { from, to } => {
                let object = self.context.target_operand(from)?;
                let destination = self.context.target_operand(to)?;
                self.counter_parts(
                    ClauseVerbAst::Move,
                    CompilerCounterOperationAst::Move,
                    None,
                    CompilerCounterAmountAst::All,
                    object,
                    Some(destination),
                    false,
                )?
            }
            SubjectVerbActionAst::MoveOneCounter { from, to } => {
                let object = self.context.target_operand(from)?;
                let destination = self.context.target_operand(to)?;
                self.counter_parts(
                    ClauseVerbAst::Move,
                    CompilerCounterOperationAst::Move,
                    None,
                    CompilerCounterAmountAst::Value(CompilerValueAst::Fixed(1)),
                    object,
                    Some(destination),
                    false,
                )?
            }
            SubjectVerbActionAst::DoubleCountersOnTarget {
                counter_type,
                target,
            } => {
                let object = self.context.target_operand(target)?;
                self.counter_parts(
                    ClauseVerbAst::Add,
                    CompilerCounterOperationAst::Double,
                    counter_type.clone(),
                    CompilerCounterAmountAst::Existing,
                    object,
                    None,
                    false,
                )?
            }
            SubjectVerbActionAst::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                ..
            } if condition.is_none() => {
                let object = self.context.target_operand(target)?;
                self.characteristic_parts(
                    CompilerCharacteristicOperationAst::AddPowerToughness,
                    object,
                    Some(power.clone()),
                    Some(toughness.clone()),
                    duration,
                )?
            }
            SubjectVerbActionAst::PumpAll {
                filter,
                power,
                toughness,
                duration,
                ..
            } => self.characteristic_parts(
                CompilerCharacteristicOperationAst::AddPowerToughness,
                CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter.clone())),
                Some(power.clone()),
                Some(toughness.clone()),
                duration,
            )?,
            SubjectVerbActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                ..
            } => {
                let object = self.context.target_operand(target)?;
                self.characteristic_parts(
                    CompilerCharacteristicOperationAst::SetPowerToughness,
                    object,
                    Some(power.clone()),
                    Some(toughness.clone()),
                    duration,
                )?
            }
            SubjectVerbActionAst::Goad { target, duration } => {
                let primary = self.context.target_operand(target)?;
                let Some(duration) = compiler_duration(duration) else {
                    return Ok(None);
                };
                (
                    ClauseVerbAst::Become,
                    Some(primary.clone()),
                    Some(duration.clone()),
                    CompilerInteractionClauseAst::Combat(CompilerCombatClauseAst {
                        operation: CompilerCombatOperationAst::Goad,
                        primary,
                        primary_role: CompilerCombatRoleAst::Affected,
                        opposing: None,
                        opposing_role: None,
                        duration: Some(duration),
                    }),
                )
            }
            SubjectVerbActionAst::Detain { target }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target } => {
                let primary = self.context.target_operand(target)?;
                let operation = match &subject_verb.action {
                    SubjectVerbActionAst::Detain { .. } => CompilerCombatOperationAst::Detain,
                    SubjectVerbActionAst::Suspect { .. } => CompilerCombatOperationAst::Suspect,
                    SubjectVerbActionAst::RemoveFromCombat { .. } => {
                        CompilerCombatOperationAst::RemoveFromCombat
                    }
                    _ => unreachable!(),
                };
                (
                    ClauseVerbAst::Become,
                    Some(primary.clone()),
                    None,
                    CompilerInteractionClauseAst::Combat(CompilerCombatClauseAst {
                        operation,
                        primary,
                        primary_role: CompilerCombatRoleAst::Affected,
                        opposing: None,
                        opposing_role: None,
                        duration: None,
                    }),
                )
            }
            _ => return Ok(None),
        };

        Ok(Some(CompilerClauseAst {
            actor: actor.clone(),
            subject: ClauseSubjectAst::Actor(actor),
            action: ClauseActionAst {
                verb,
                polarity: ClausePolarityAst::Positive,
            },
            object: object.as_ref().map(clause_object),
            quantity: None,
            destination: None,
            duration,
            condition: None,
            bindings: Vec::new(),
            complements: Vec::new(),
            library: None,
            object_action: None,
            interaction: Some(interaction),
            provenance: None,
        }))
    }

    fn effect_result(
        &mut self,
        role: ReferenceRole,
    ) -> Result<crate::model::SymbolReference, SymbolResolutionError> {
        self.context
            .bind_selection(role, ObjectDomain::EffectResult, Cardinality::Any)
    }

    fn counter_parts(
        &mut self,
        verb: ClauseVerbAst,
        operation: CompilerCounterOperationAst,
        counter_type: Option<crate::object::CounterType>,
        amount: CompilerCounterAmountAst,
        object: CompilerObjectOperandAst,
        destination: Option<CompilerObjectOperandAst>,
        distributed: bool,
    ) -> Result<
        (
            ClauseVerbAst,
            Option<CompilerObjectOperandAst>,
            Option<crate::model::ClauseDurationAst>,
            CompilerInteractionClauseAst,
        ),
        SymbolResolutionError,
    > {
        let affected = self.effect_result(ReferenceRole::Affected)?;
        Ok((
            verb,
            Some(object.clone()),
            None,
            CompilerInteractionClauseAst::Counter(CompilerCounterClauseAst {
                operation,
                counter_type,
                amount,
                object,
                destination,
                distributed,
                affected,
            }),
        ))
    }

    fn characteristic_parts(
        &mut self,
        operation: CompilerCharacteristicOperationAst,
        object: CompilerObjectOperandAst,
        power: Option<crate::effect::Value>,
        toughness: Option<crate::effect::Value>,
        duration: &Until,
    ) -> Result<
        (
            ClauseVerbAst,
            Option<CompilerObjectOperandAst>,
            Option<crate::model::ClauseDurationAst>,
            CompilerInteractionClauseAst,
        ),
        SymbolResolutionError,
    > {
        let duration = compiler_duration(duration);
        let affected = self.effect_result(ReferenceRole::Affected)?;
        Ok((
            match operation {
                CompilerCharacteristicOperationAst::AddPowerToughness => ClauseVerbAst::Add,
                _ => ClauseVerbAst::Become,
            },
            Some(object.clone()),
            duration.clone(),
            CompilerInteractionClauseAst::Characteristic(CompilerCharacteristicClauseAst {
                mode: CompilerModificationModeAst::Continuous,
                layer: match operation {
                    CompilerCharacteristicOperationAst::SetPowerToughness
                    | CompilerCharacteristicOperationAst::SetPower => {
                        ContinuousLayerAst::PowerToughnessSet
                    }
                    _ => ContinuousLayerAst::PowerToughnessModify,
                },
                operation,
                object,
                power: power.map(CompilerValueAst::Dynamic),
                toughness: toughness.map(CompilerValueAst::Dynamic),
                duration,
                affected,
            }),
        ))
    }
}

pub(crate) fn migrate_interaction_clauses(
    items: &mut [ParsedCardItem],
    context: &mut SemanticMigrationContext<'_>,
) -> Result<(), SymbolResolutionError> {
    let mut migration = InteractionMigration { context };
    for item in items {
        migrate_item(item, &mut migration)?;
    }
    Ok(())
}

fn migrate_item(
    item: &mut ParsedCardItem,
    migration: &mut InteractionMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    match item {
        ParsedCardItem::Line(line) => {
            for chunk in &mut line.chunks {
                migrate_line_chunk(chunk, migration)?;
            }
            if let Some(ability) = &mut line.semantic_facts.triggered_ability.compiler_ability {
                migration.migrate_effects(&mut ability.effects)?;
                for program in &mut ability.program.programs {
                    migration.migrate_effects(&mut program.effects)?;
                }
            }
        }
        ParsedCardItem::Modal(modal) => migrate_modal(modal, migration)?,
        ParsedCardItem::LevelAbility(level) => {
            for item in &mut level.items {
                if let ParsedLevelAbilityItemAst::ActivatedAbility(activated) = item {
                    migrate_line_chunk(&mut activated.chunk, migration)?;
                }
            }
        }
    }
    Ok(())
}

fn migrate_line_chunk(
    chunk: &mut LineAst,
    migration: &mut InteractionMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                migrate_line_chunk(chunk, migration)?;
            }
        }
        LineAst::Ability(ability) => {
            if let Some(effects) = &mut ability.effects_ast {
                migration.migrate_effects(effects)?;
            }
        }
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects }
        | LineAst::GiftKeyword { effects, .. }
        | LineAst::OptionalCostWithCastTrigger { effects, .. } => {
            migration.migrate_effects(effects)?;
        }
        LineAst::AdditionalCostChoice { options } => {
            for option in options {
                migration.migrate_effects(&mut option.effects)?;
            }
        }
        LineAst::Abilities(_)
        | LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::OptionalCost(_)
        | LineAst::AlternativeCastingMethod(_) => {}
    }
    Ok(())
}

fn migrate_modal(
    modal: &mut ParsedModalAst,
    migration: &mut InteractionMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    migration.migrate_effects(&mut modal.header.prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_suffix_effects_ast)?;
    for mode in &mut modal.modes {
        migration.migrate_effects(&mut mode.effects_ast)?;
    }
    Ok(())
}

fn clause_object(operand: &CompilerObjectOperandAst) -> ClauseObjectAst {
    match operand {
        CompilerObjectOperandAst::Source => ClauseObjectAst::Subject(ClauseSubjectAst::Source),
        CompilerObjectOperandAst::Selection(selection) => {
            ClauseObjectAst::Selection(selection.clone())
        }
        CompilerObjectOperandAst::Reference(reference) => ClauseObjectAst::Reference(*reference),
        CompilerObjectOperandAst::Filter(filter) => ClauseObjectAst::Filter(filter.clone()),
    }
}

fn compiler_duration(duration: &Until) -> Option<crate::model::ClauseDurationAst> {
    match duration {
        Until::Forever => Some(crate::model::ClauseDurationAst::Permanent),
        Until::EndOfTurn => Some(crate::model::ClauseDurationAst::UntilEndOfTurn),
        Until::YourNextTurn | Until::YourNextTurnEnd => {
            Some(crate::model::ClauseDurationAst::UntilNextTurn)
        }
        Until::EndOfCombat => Some(crate::model::ClauseDurationAst::UntilEndOfCombat),
        Until::TurnsPass(value) => Some(crate::model::ClauseDurationAst::ForTurns(
            CompilerValueAst::Dynamic(value.clone()),
        )),
        _ => None,
    }
}
