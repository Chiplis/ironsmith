//! Finite PR-26 adapter from object-action leaves to common compiler clauses.

use crate::effect::Until;
use crate::model::ast::{EffectAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActionAst, ClauseActorAst, ClauseDestinationAst, ClauseDestinationRelationAst,
    ClauseDurationAst, ClauseObjectAst, ClausePolarityAst, ClauseSubjectAst, ClauseVerbAst,
    ClauseZonePlacementAst, CompilerClauseAst,
};
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::object_action_clauses::{
    CompilerAttachmentClauseAst, CompilerControlClauseAst, CompilerControllerAst,
    CompilerCopyModificationsAst, CompilerCreationClauseAst, CompilerCreationKindAst,
    CompilerDelayedDispositionAst, CompilerEntryStateAst, CompilerMovementClauseAst,
    CompilerObjectActionClauseAst, CompilerObjectOperandAst,
};
use crate::model::parse_types::ReturnControllerAst;
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::symbols::{Cardinality, ReferenceRole, SymbolResolutionError};
use crate::model::visit::for_each_nested_effect_vec_mut;
use crate::runtime_backend::front_end::semantic_migration_context::SemanticMigrationContext;
use crate::zone::Zone;

struct ObjectActionMigration<'migration, 'symbols> {
    context: &'migration mut SemanticMigrationContext<'symbols>,
}

type ObjectActionParts = (
    ClauseVerbAst,
    Option<CompilerObjectOperandAst>,
    Option<ClauseDestinationAst>,
    Option<ClauseDurationAst>,
    CompilerObjectActionClauseAst,
);

impl<'migration, 'symbols> ObjectActionMigration<'migration, 'symbols> {
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
        if let Some(clause) = self.object_action_clause(effect)? {
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

    fn object_action_clause(
        &mut self,
        effect: &EffectAst,
    ) -> Result<Option<CompilerClauseAst>, SymbolResolutionError> {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return Ok(None);
        };
        let actor = super::library_clause_migration::compiler_actor(subject_verb.subject.player);
        let (verb, object, destination, duration, action) = match &subject_verb.action {
            SubjectVerbActionAst::MayMoveToZone { target, zone } => Self::movement(
                ClauseVerbAst::Move,
                self.context.target_operand(target)?,
                *zone,
                ClauseZonePlacementAst::Default,
                false,
                false,
                false,
            ),
            SubjectVerbActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak,
                ..
            } => {
                let object = self.context.target_operand(target)?;
                let controller = compiler_controller(*controller);
                let destination = zone_destination(
                    Zone::Battlefield,
                    if *tapped {
                        ClauseZonePlacementAst::Tapped
                    } else {
                        ClauseZonePlacementAst::Default
                    },
                    controller_actor(controller, &actor),
                );
                (
                    ClauseVerbAst::Put,
                    Some(object.clone()),
                    Some(destination.clone()),
                    None,
                    CompilerObjectActionClauseAst::Movement(CompilerMovementClauseAst {
                        object,
                        source_zones: Vec::new(),
                        source_top_only: false,
                        destination,
                        controller,
                        state: CompilerEntryStateAst {
                            tapped: *tapped,
                            cloaked: *cloak,
                            ..default_entry_state()
                        },
                        all: false,
                        random: false,
                        replacement: false,
                    }),
                )
            }
            SubjectVerbActionAst::MoveToZone {
                target,
                source_top_only,
                zone,
                to_top,
                library_order,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                battlefield_transformed,
                attached_to,
                all,
                ..
            } => {
                let object = self.context.target_operand(target)?;
                let controller = compiler_controller(*battlefield_controller);
                let destination = zone_destination(
                    *zone,
                    if *to_top {
                        ClauseZonePlacementAst::Top
                    } else if library_order.is_some() {
                        ClauseZonePlacementAst::Bottom
                    } else if *battlefield_tapped {
                        ClauseZonePlacementAst::Tapped
                    } else if *battlefield_face_down {
                        ClauseZonePlacementAst::FaceDown
                    } else {
                        ClauseZonePlacementAst::Default
                    },
                    controller_actor(controller, &actor),
                );
                (
                    ClauseVerbAst::Move,
                    Some(object.clone()),
                    Some(destination.clone()),
                    None,
                    CompilerObjectActionClauseAst::Movement(CompilerMovementClauseAst {
                        object,
                        source_zones: Vec::new(),
                        source_top_only: *source_top_only,
                        destination,
                        controller,
                        state: CompilerEntryStateAst {
                            tapped: *battlefield_tapped,
                            attacking: *battlefield_attacking,
                            attack_target:
                                battlefield_attack_target_player_or_planeswalker_controlled_by
                                    .as_ref()
                                    .copied()
                                    .map(super::library_clause_migration::compiler_actor),
                            face_down: *battlefield_face_down,
                            transformed: *battlefield_transformed,
                            cloaked: false,
                            attached_to: attached_to
                                .as_ref()
                                .map(|target| self.context.target_operand(target))
                                .transpose()?,
                        },
                        all: *all,
                        random: false,
                        replacement: false,
                    }),
                )
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                from_graveyard_or_exile,
                tapped,
                transformed,
                controller,
                top_only,
                as_aura,
                ..
            } if as_aura.is_none() => {
                let object = self.context.target_operand(target)?;
                let controller = compiler_controller(*controller);
                let destination = zone_destination(
                    Zone::Battlefield,
                    if *tapped {
                        ClauseZonePlacementAst::Tapped
                    } else {
                        ClauseZonePlacementAst::Default
                    },
                    controller_actor(controller, &actor),
                );
                (
                    ClauseVerbAst::Return,
                    Some(object.clone()),
                    Some(destination.clone()),
                    None,
                    CompilerObjectActionClauseAst::Movement(CompilerMovementClauseAst {
                        object,
                        source_zones: if *from_graveyard_or_exile {
                            vec![Zone::Graveyard, Zone::Exile]
                        } else {
                            vec![Zone::Graveyard]
                        },
                        source_top_only: *top_only,
                        destination,
                        controller,
                        state: CompilerEntryStateAst {
                            tapped: *tapped,
                            transformed: *transformed,
                            ..default_entry_state()
                        },
                        all: false,
                        random: false,
                        replacement: false,
                    }),
                )
            }
            SubjectVerbActionAst::Exile {
                target,
                face_down,
                source_top_only,
                ..
            } => Self::movement(
                ClauseVerbAst::Exile,
                self.context.target_operand(target)?,
                Zone::Exile,
                if *face_down {
                    ClauseZonePlacementAst::FaceDown
                } else {
                    ClauseZonePlacementAst::Default
                },
                false,
                false,
                *source_top_only,
            ),
            SubjectVerbActionAst::ExileAll { filter, face_down } => Self::movement(
                ClauseVerbAst::Exile,
                CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter.clone())),
                Zone::Exile,
                if *face_down {
                    ClauseZonePlacementAst::FaceDown
                } else {
                    ClauseZonePlacementAst::Default
                },
                true,
                false,
                false,
            ),
            SubjectVerbActionAst::ReturnToHand { target, random, .. } => Self::movement(
                ClauseVerbAst::Return,
                self.context.target_operand(target)?,
                Zone::Hand,
                ClauseZonePlacementAst::Default,
                false,
                *random,
                false,
            ),
            SubjectVerbActionAst::ReturnAllToHand { filter, .. }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter } => Self::movement(
                ClauseVerbAst::Return,
                CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter.clone())),
                Zone::Hand,
                ClauseZonePlacementAst::Default,
                true,
                false,
                false,
            ),
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target, all, .. } => Self::movement(
                ClauseVerbAst::Shuffle,
                self.context.target_operand(target)?,
                Zone::Library,
                ClauseZonePlacementAst::Shuffled,
                *all,
                false,
                false,
            ),
            SubjectVerbActionAst::Attach { object, target } => {
                let attachment = self.context.target_operand(object)?;
                let target = self.context.target_operand(target)?;
                (
                    ClauseVerbAst::Attach,
                    Some(attachment.clone()),
                    None,
                    None,
                    CompilerObjectActionClauseAst::Attachment(CompilerAttachmentClauseAst {
                        attachment,
                        target: Some(target),
                        legality: None,
                        detach: false,
                    }),
                )
            }
            SubjectVerbActionAst::Unattach { object } => {
                let attachment = self.context.target_operand(object)?;
                (
                    ClauseVerbAst::Attach,
                    Some(attachment.clone()),
                    None,
                    None,
                    CompilerObjectActionClauseAst::Attachment(CompilerAttachmentClauseAst {
                        attachment,
                        target: None,
                        legality: None,
                        detach: true,
                    }),
                )
            }
            SubjectVerbActionAst::Enchant { filter } => (
                ClauseVerbAst::Attach,
                Some(CompilerObjectOperandAst::Source),
                None,
                None,
                CompilerObjectActionClauseAst::Attachment(CompilerAttachmentClauseAst {
                    attachment: CompilerObjectOperandAst::Source,
                    target: None,
                    legality: Some(filter.clone()),
                    detach: false,
                }),
            ),
            SubjectVerbActionAst::GainControl {
                target,
                duration,
                condition,
                ..
            } if condition.is_none() => {
                let Some(duration) = compiler_duration(duration) else {
                    return Ok(None);
                };
                let object = self.context.target_operand(target)?;
                (
                    ClauseVerbAst::Control,
                    Some(object.clone()),
                    None,
                    Some(duration.clone()),
                    CompilerObjectActionClauseAst::Control(CompilerControlClauseAst {
                        object,
                        controller: actor.clone(),
                        duration: Some(duration),
                        exchange_with: None,
                    }),
                )
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                name,
                definition,
                count,
                dynamic_power_toughness,
                player,
                attached_to,
                tapped,
                attacking,
                attack_target_player,
                exile_at_end_of_combat,
                sacrifice_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                granted_abilities,
                ..
            } if granted_abilities.is_empty() => {
                let controller = super::library_clause_migration::compiler_actor(*player);
                let result =
                    self.context
                        .bind_object(None, ReferenceRole::Created, Cardinality::Any)?;
                (
                    ClauseVerbAst::Create,
                    None,
                    Some(zone_destination(
                        Zone::Battlefield,
                        ClauseZonePlacementAst::Default,
                        Some(controller.clone()),
                    )),
                    None,
                    CompilerObjectActionClauseAst::Creation(CompilerCreationClauseAst {
                        kind: CompilerCreationKindAst::Token {
                            name: name.clone(),
                            definition: definition.clone(),
                            dynamic_power_toughness: dynamic_power_toughness.clone().map(
                                |(power, toughness)| {
                                    (
                                        CompilerValueAst::Dynamic(power),
                                        CompilerValueAst::Dynamic(toughness),
                                    )
                                },
                            ),
                            granted_abilities: Vec::new(),
                        },
                        count: CompilerValueAst::Dynamic(count.clone()),
                        controller,
                        state: CompilerEntryStateAst {
                            tapped: *tapped,
                            attacking: *attacking,
                            attack_target: attack_target_player
                                .as_ref()
                                .copied()
                                .map(super::library_clause_migration::compiler_actor),
                            attached_to: attached_to
                                .as_ref()
                                .map(|target| self.context.target_operand(target))
                                .transpose()?,
                            ..default_entry_state()
                        },
                        modifications: CompilerCopyModificationsAst::default(),
                        delayed_dispositions: delayed_dispositions(
                            *exile_at_end_of_combat,
                            *sacrifice_at_end_of_combat,
                            *exile_at_next_end_step,
                            *sacrifice_at_next_end_step,
                        ),
                        result,
                    }),
                )
            }
            SubjectVerbActionAst::CopySpell {
                target,
                count,
                player,
                may_choose_new_targets,
                removed_supertypes,
                set_colors,
                added_card_types,
                added_subtypes,
                set_base_power_toughness,
                ..
            } => {
                let source = self.context.target_operand(target)?;
                let result =
                    self.context
                        .bind_object(None, ReferenceRole::Copied, Cardinality::Any)?;
                (
                    ClauseVerbAst::Copy,
                    Some(source.clone()),
                    None,
                    None,
                    CompilerObjectActionClauseAst::Creation(CompilerCreationClauseAst {
                        kind: CompilerCreationKindAst::SpellCopy {
                            source,
                            may_choose_new_targets: *may_choose_new_targets,
                        },
                        count: CompilerValueAst::Dynamic(count.clone()),
                        controller: super::library_clause_migration::compiler_actor(*player),
                        state: default_entry_state(),
                        modifications: CompilerCopyModificationsAst {
                            set_colors: set_colors.clone(),
                            add_card_types: added_card_types.clone(),
                            add_subtypes: added_subtypes.clone(),
                            remove_supertypes: removed_supertypes.clone(),
                            set_base_power_toughness: set_base_power_toughness.as_ref().map(
                                |(power, toughness)| {
                                    (
                                        CompilerValueAst::Fixed(*power),
                                        CompilerValueAst::Fixed(*toughness),
                                    )
                                },
                            ),
                            ..CompilerCopyModificationsAst::default()
                        },
                        delayed_dispositions: Vec::new(),
                        result,
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
            destination,
            duration,
            condition: None,
            bindings: Vec::new(),
            complements: Vec::new(),
            library: None,
            object_action: Some(action),
            interaction: None,
            resource_choice: None,
            permission: None,
            provenance: None,
        }))
    }

    fn movement(
        verb: ClauseVerbAst,
        object: CompilerObjectOperandAst,
        zone: Zone,
        placement: ClauseZonePlacementAst,
        all: bool,
        random: bool,
        source_top_only: bool,
    ) -> ObjectActionParts {
        let destination = zone_destination(zone, placement, None);
        (
            verb,
            Some(object.clone()),
            Some(destination.clone()),
            None,
            CompilerObjectActionClauseAst::Movement(CompilerMovementClauseAst {
                object,
                source_zones: Vec::new(),
                source_top_only,
                destination,
                controller: CompilerControllerAst::Preserve,
                state: default_entry_state(),
                all,
                random,
                replacement: false,
            }),
        )
    }
}

pub(crate) fn migrate_object_action_clauses(
    items: &mut [ParsedCardItem],
    context: &mut SemanticMigrationContext<'_>,
) -> Result<(), SymbolResolutionError> {
    let mut migration = ObjectActionMigration { context };
    for item in items {
        migrate_item(item, &mut migration)?;
    }
    Ok(())
}

fn migrate_item(
    item: &mut ParsedCardItem,
    migration: &mut ObjectActionMigration<'_, '_>,
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
    migration: &mut ObjectActionMigration<'_, '_>,
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
    migration: &mut ObjectActionMigration<'_, '_>,
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

fn zone_destination(
    zone: Zone,
    placement: ClauseZonePlacementAst,
    controller: Option<ClauseActorAst>,
) -> ClauseDestinationAst {
    ClauseDestinationAst {
        relation: ClauseDestinationRelationAst::To,
        zone,
        placement,
        controller,
    }
}

fn compiler_controller(controller: ReturnControllerAst) -> CompilerControllerAst {
    match controller {
        ReturnControllerAst::Preserve => CompilerControllerAst::Preserve,
        ReturnControllerAst::Owner => CompilerControllerAst::Owner,
        ReturnControllerAst::You => CompilerControllerAst::SourceController,
    }
}

fn controller_actor(
    controller: CompilerControllerAst,
    actor: &ClauseActorAst,
) -> Option<ClauseActorAst> {
    match controller {
        CompilerControllerAst::Actor => Some(actor.clone()),
        CompilerControllerAst::SourceController => Some(ClauseActorAst::SourceController),
        CompilerControllerAst::Preserve | CompilerControllerAst::Owner => None,
    }
}

fn default_entry_state() -> CompilerEntryStateAst {
    CompilerEntryStateAst {
        tapped: false,
        attacking: false,
        attack_target: None,
        face_down: false,
        transformed: false,
        cloaked: false,
        attached_to: None,
    }
}

fn compiler_duration(duration: &Until) -> Option<ClauseDurationAst> {
    match duration {
        Until::Forever => Some(ClauseDurationAst::Permanent),
        Until::EndOfTurn => Some(ClauseDurationAst::UntilEndOfTurn),
        Until::YourNextTurn | Until::YourNextTurnEnd => Some(ClauseDurationAst::UntilNextTurn),
        Until::EndOfCombat => Some(ClauseDurationAst::UntilEndOfCombat),
        Until::TurnsPass(value) => Some(ClauseDurationAst::ForTurns(CompilerValueAst::Dynamic(
            value.clone(),
        ))),
        _ => None,
    }
}

fn delayed_dispositions(
    exile_end_of_combat: bool,
    sacrifice_end_of_combat: bool,
    exile_next_end_step: bool,
    sacrifice_next_end_step: bool,
) -> Vec<CompilerDelayedDispositionAst> {
    let mut dispositions = Vec::new();
    if exile_end_of_combat {
        dispositions.push(CompilerDelayedDispositionAst::ExileEndOfCombat);
    }
    if sacrifice_end_of_combat {
        dispositions.push(CompilerDelayedDispositionAst::SacrificeEndOfCombat);
    }
    if exile_next_end_step {
        dispositions.push(CompilerDelayedDispositionAst::ExileNextEndStep);
    }
    if sacrifice_next_end_step {
        dispositions.push(CompilerDelayedDispositionAst::SacrificeNextEndStep);
    }
    dispositions
}
