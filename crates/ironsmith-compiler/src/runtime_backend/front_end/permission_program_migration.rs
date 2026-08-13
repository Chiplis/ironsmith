//! Finite PR-29 adapter for casting permissions and document programs.
//!
//! This boundary consumes only already typed legacy AST nodes.  It performs
//! no token, source-text, or card-name recognition and does not contain a
//! whole-program recipe registry.

use std::ops::ControlFlow;

use crate::effect::{Restriction, RestrictionStart, Until};
use crate::model::ast::{EffectAst, SubjectVerbActionAst, SubjectVerbEffectAst};
use crate::model::clauses::{
    ClauseActionAst, ClauseActorAst, ClauseObjectAst, ClausePolarityAst, ClauseSubjectAst,
    ClauseVerbAst, CompilerClauseAst,
};
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::document_program::{
    CompilerDocumentProgramAst, CompilerDocumentStatementAst, CompilerStatementEdgeAst,
    CompilerStatementEdgeKindAst, CompilerStatementId,
};
use crate::model::legality::TimingWindowAst;
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::permission_clauses::{
    CompilerAlternativeCastAst, CompilerCastingActionAst, CompilerCastingCostAst,
    CompilerCastingOriginAst, CompilerCastingPaymentAst, CompilerCostAdjustmentAst,
    CompilerManaFlexibilityAst, CompilerPermissionActorAst, CompilerPermissionClauseAst,
    CompilerPermissionDispositionAst, CompilerPermissionExpirationAst,
    CompilerPermissionFrequencyAst, CompilerPermissionStartAst,
};
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError,
    SymbolScopeKind,
};
use crate::model::visit::{
    SemanticVisitor, for_each_nested_effect_vec_mut, visit_effect_tree,
};
use crate::runtime_backend::front_end::semantic_migration_context::SemanticMigrationContext;
use crate::tag::TagKey;
use crate::target::ObjectFilter;
use crate::zone::Zone;

struct PermissionProgramMigration<'migration, 'symbols> {
    context: &'migration mut SemanticMigrationContext<'symbols>,
}

impl<'migration, 'symbols> PermissionProgramMigration<'migration, 'symbols> {
    fn migrate_effects(
        &mut self,
        effects: &mut Vec<EffectAst>,
    ) -> Result<(), SymbolResolutionError> {
        for effect in effects.iter_mut() {
            self.migrate_effect(effect)?;
        }
        self.compose_source_sentences(effects)
    }

    fn migrate_effect(&mut self, effect: &mut EffectAst) -> Result<(), SymbolResolutionError> {
        if let Some(clause) = self.permission_clause(effect)? {
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

    fn permission_clause(
        &mut self,
        effect: &EffectAst,
    ) -> Result<Option<CompilerClauseAst>, SymbolResolutionError> {
        let permission = match effect {
            EffectAst::SubjectVerb(subject_verb) => {
                self.subject_verb_permission(subject_verb)?
            }
            EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                zone_owner,
                filter,
                zone,
                payment,
            } => Some(CompilerPermissionClauseAst {
                disposition: CompilerPermissionDispositionAst::Permit,
                action: CompilerCastingActionAst::CastSpell,
                actor: compiler_permission_actor(*player),
                object: spell_filter(filter.clone()),
                qualification: None,
                origin: CompilerCastingOriginAst::Zones {
                    zones: vec![*zone],
                    owner: Some(super::library_clause_migration::compiler_actor(*zone_owner)),
                },
                timing: None,
                cost: CompilerCastingCostAst {
                    payment: compiler_payment(payment),
                    adjustments: Vec::new(),
                    mana_flexibility: CompilerManaFlexibilityAst::AsWritten,
                },
                starts: CompilerPermissionStartAst::Immediate,
                expiration: CompilerPermissionExpirationAst::Immediate,
                frequency: CompilerPermissionFrequencyAst::Once,
                linked_object: None,
                as_copy: false,
                lands_enter_tapped: false,
            }),
            _ => None,
        };
        Ok(permission.map(common_permission_clause))
    }

    fn subject_verb_permission(
        &mut self,
        subject_verb: &SubjectVerbEffectAst,
    ) -> Result<Option<CompilerPermissionClauseAst>, SymbolResolutionError> {
        let subject_actor =
            super::library_clause_migration::compiler_actor(subject_verb.subject.player);
        let permission = match &subject_verb.action {
            SubjectVerbActionAst::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
                ..
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                Some(CompilerPermissionClauseAst {
                    disposition: CompilerPermissionDispositionAst::Execute,
                    action: casting_action(*allow_land),
                    actor: compiler_permission_actor(*player),
                    object,
                    qualification: None,
                    origin: CompilerCastingOriginAst::CurrentZone,
                    timing: None,
                    cost: CompilerCastingCostAst {
                        payment: if *without_paying_mana_cost {
                            CompilerCastingPaymentAst::WithoutPayingManaCost
                        } else {
                            CompilerCastingPaymentAst::PrintedCost
                        },
                        adjustments: casting_adjustments(
                            additional_mana_cost.as_ref(),
                            cost_reduction.as_ref(),
                        ),
                        mana_flexibility: mana_flexibility(*mana_spend_mode),
                    },
                    starts: CompilerPermissionStartAst::Immediate,
                    expiration: CompilerPermissionExpirationAst::Immediate,
                    frequency: CompilerPermissionFrequencyAst::Once,
                    linked_object: Some(reference),
                    as_copy: *as_copy,
                    lands_enter_tapped: false,
                })
            }
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
                free_cast_from_current_zone,
                until_source_exiles_another,
                max_plays,
                ..
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                let origin = if *while_on_top_of_library {
                    CompilerCastingOriginAst::TopOfLibrary { owner: None }
                } else if *free_cast_from_current_zone {
                    CompilerCastingOriginAst::CurrentZone
                } else {
                    CompilerCastingOriginAst::ExiledWithSource
                };
                let mut bounds = vec![CompilerPermissionExpirationAst::UntilEndOfTurn];
                if *while_on_top_of_library {
                    bounds.push(CompilerPermissionExpirationAst::WhileOnTopOfLibrary);
                }
                if *until_source_exiles_another {
                    bounds.push(CompilerPermissionExpirationAst::UntilSourceExilesAnother);
                }
                Some(tagged_permission(
                    *player,
                    *allow_land,
                    object,
                    reference,
                    origin,
                    CompilerCastingCostAst {
                        payment: if *without_paying_mana_cost {
                            CompilerCastingPaymentAst::WithoutPayingManaCost
                        } else {
                            CompilerCastingPaymentAst::PrintedCost
                        },
                        adjustments: Vec::new(),
                        mana_flexibility: mana_flexibility(*allow_any_color_for_cast),
                    },
                    bounded_expiration(bounds),
                    permission_frequency(*max_plays),
                    false,
                ))
            }
            SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                Some(tagged_permission(
                    *player,
                    false,
                    object,
                    reference,
                    CompilerCastingOriginAst::ExiledWithSource,
                    CompilerCastingCostAst {
                        payment: CompilerCastingPaymentAst::PayLifeByManaValue,
                        adjustments: Vec::new(),
                        mana_flexibility: CompilerManaFlexibilityAst::AsWritten,
                    },
                    CompilerPermissionExpirationAst::UntilEndOfTurn,
                    CompilerPermissionFrequencyAst::Unbounded,
                    false,
                ))
            }
            SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step,
                max_plays,
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                Some(tagged_permission(
                    *player,
                    *allow_land,
                    object,
                    reference,
                    CompilerCastingOriginAst::ExiledWithSource,
                    printed_cost(*allow_any_color_for_cast),
                    if *until_next_end_step {
                        CompilerPermissionExpirationAst::UntilYourNextEndStep
                    } else {
                        CompilerPermissionExpirationAst::UntilYourNextTurn
                    },
                    permission_frequency(*max_plays),
                    false,
                ))
            }
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                during_turns_counter_put_on_source,
                spell_cost_increase,
                lands_enter_tapped,
                filter,
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                let mut bounds = vec![CompilerPermissionExpirationAst::WhileExiled];
                if let Some(counter) = during_turns_counter_put_on_source {
                    bounds.push(
                        CompilerPermissionExpirationAst::DuringTurnsCounterPutOnSource(
                            counter.clone(),
                        ),
                    );
                }
                let mut permission = tagged_permission(
                    *player,
                    *allow_land,
                    object,
                    reference,
                    CompilerCastingOriginAst::ExiledWithSource,
                    CompilerCastingCostAst {
                        payment: if *without_paying_mana_cost {
                            CompilerCastingPaymentAst::WithoutPayingManaCost
                        } else {
                            CompilerCastingPaymentAst::PrintedCost
                        },
                        adjustments: spell_cost_increase
                            .as_ref()
                            .map(|cost| CompilerCostAdjustmentAst::IncreaseMana(cost.clone()))
                            .into_iter()
                            .collect(),
                        mana_flexibility: mana_flexibility(*allow_any_color_for_cast),
                    },
                    bounded_expiration(bounds),
                    CompilerPermissionFrequencyAst::Unbounded,
                    *lands_enter_tapped,
                );
                permission.qualification = filter.clone().map(CompilerFilterAst::Card);
                Some(permission)
            }
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                ..
            } => {
                let (object, reference) = self.tagged_object(tag)?;
                Some(tagged_permission(
                    *player,
                    *allow_land,
                    object,
                    reference,
                    CompilerCastingOriginAst::CurrentZone,
                    printed_cost(*allow_any_color_for_cast),
                    CompilerPermissionExpirationAst::WhileYouControlSource,
                    CompilerPermissionFrequencyAst::Unbounded,
                    false,
                ))
            }
            SubjectVerbActionAst::PlayFromGraveyardUntilEot => Some(
                CompilerPermissionClauseAst {
                    disposition: CompilerPermissionDispositionAst::Permit,
                    action: CompilerCastingActionAst::CastOrPlay,
                    actor: CompilerPermissionActorAst::Actor(subject_actor.clone()),
                    object: card_filter(ObjectFilter::default()),
                    qualification: None,
                    origin: CompilerCastingOriginAst::Zones {
                        zones: vec![Zone::Graveyard],
                        owner: Some(subject_actor.clone()),
                    },
                    timing: None,
                    cost: printed_cost(ironsmith_core::value_model::ManaSpendMode::Normal),
                    starts: CompilerPermissionStartAst::Immediate,
                    expiration: CompilerPermissionExpirationAst::UntilEndOfTurn,
                    frequency: CompilerPermissionFrequencyAst::Unbounded,
                    linked_object: None,
                    as_copy: false,
                    lands_enter_tapped: false,
                },
            ),
            SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, reduction } => Some(
                cost_modification_permission(
                    subject_actor.clone(),
                    filter.clone(),
                    CompilerCostAdjustmentAst::ReduceMana(reduction.clone()),
                    CompilerPermissionExpirationAst::UntilEndOfTurn,
                    CompilerPermissionFrequencyAst::Once,
                ),
            ),
            SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
                next_only,
            } => permission_expiration(duration).map(|expiration| {
                cost_modification_permission(
                    subject_actor.clone(),
                    filter.clone(),
                    CompilerCostAdjustmentAst::ReduceValue(CompilerValueAst::Dynamic(
                        reduction.clone(),
                    )),
                    expiration,
                    if *next_only {
                        CompilerPermissionFrequencyAst::Once
                    } else {
                        CompilerPermissionFrequencyAst::Unbounded
                    },
                )
            }),
            SubjectVerbActionAst::Cant {
                restriction,
                duration,
                start,
                condition,
                ..
            } if condition.is_none() => self.restriction_permission(
                restriction,
                duration,
                start,
                subject_actor,
            ),
            _ => None,
        };
        Ok(permission)
    }

    fn tagged_object(
        &mut self,
        tag: &TagKey,
    ) -> Result<(CompilerObjectOperandAst, SymbolReference), SymbolResolutionError> {
        let reference = if let Some(reference) = self.context.object_reference(tag) {
            reference
        } else {
            self.context.bind_tagged(
                Some(tag.clone()),
                ReferenceRole::Affected,
                Cardinality::Any,
                ObjectDomain::Card,
            )?
        };
        Ok((CompilerObjectOperandAst::Reference(reference), reference))
    }

    fn restriction_permission(
        &self,
        restriction: &Restriction,
        duration: &Until,
        start: &RestrictionStart,
        fallback_actor: ClauseActorAst,
    ) -> Option<CompilerPermissionClauseAst> {
        let expiration = permission_expiration(duration)?;
        let starts = match start {
            RestrictionStart::Immediate => CompilerPermissionStartAst::Immediate,
            RestrictionStart::NextTurn(player) => {
                CompilerPermissionStartAst::NextTurn(player.clone())
            }
        };
        let (disposition, action, actor, object, timing, frequency) = match restriction {
            Restriction::CastSpellsMatching(players, filter) => (
                CompilerPermissionDispositionAst::Prohibit,
                CompilerCastingActionAst::CastSpell,
                CompilerPermissionActorAst::Players(players.clone()),
                spell_filter(filter.clone()),
                None,
                CompilerPermissionFrequencyAst::Unbounded,
            ),
            Restriction::CastSpellsOnlyAsSorcery(players) => (
                CompilerPermissionDispositionAst::Restrict,
                CompilerCastingActionAst::CastSpell,
                CompilerPermissionActorAst::Players(players.clone()),
                spell_filter(ObjectFilter::default()),
                Some(TimingWindowAst::SorcerySpeed),
                CompilerPermissionFrequencyAst::Unbounded,
            ),
            Restriction::CastMoreThanOneSpellEachTurn(players, filter) => (
                CompilerPermissionDispositionAst::Prohibit,
                CompilerCastingActionAst::CastSpell,
                CompilerPermissionActorAst::Players(players.clone()),
                spell_filter(filter.clone()),
                None,
                CompilerPermissionFrequencyAst::MoreThanOnePerTurn,
            ),
            Restriction::ActivateNonManaAbilities(players) => (
                CompilerPermissionDispositionAst::Prohibit,
                CompilerCastingActionAst::ActivateAbility,
                CompilerPermissionActorAst::Players(players.clone()),
                object_filter(ObjectFilter::default()),
                None,
                CompilerPermissionFrequencyAst::Unbounded,
            ),
            Restriction::ActivateAbilitiesOf(filter)
            | Restriction::ActivateTapAbilitiesOf(filter)
            | Restriction::ActivateNonManaAbilitiesOf(filter) => (
                CompilerPermissionDispositionAst::Prohibit,
                CompilerCastingActionAst::ActivateAbility,
                CompilerPermissionActorAst::Actor(fallback_actor),
                object_filter(filter.clone()),
                None,
                CompilerPermissionFrequencyAst::Unbounded,
            ),
            _ => return None,
        };
        Some(CompilerPermissionClauseAst {
            disposition,
            action,
            actor,
            object,
            qualification: None,
            origin: CompilerCastingOriginAst::Default,
            timing,
            cost: printed_cost(ironsmith_core::value_model::ManaSpendMode::Normal),
            starts,
            expiration,
            frequency,
            linked_object: None,
            as_copy: false,
            lands_enter_tapped: false,
        })
    }

    fn compose_source_sentences(
        &mut self,
        effects: &mut Vec<EffectAst>,
    ) -> Result<(), SymbolResolutionError> {
        let mut input = std::mem::take(effects).into_iter().peekable();
        while let Some(effect) = input.next() {
            let EffectAst::SourceSentence {
                effects: statement_effects,
                leading_then,
                starting_with_controller,
            } = effect
            else {
                effects.push(effect);
                continue;
            };
            let mut source_sentences = vec![(
                statement_effects,
                leading_then,
                starting_with_controller,
            )];
            while matches!(input.peek(), Some(EffectAst::SourceSentence { .. })) {
                let Some(EffectAst::SourceSentence {
                    effects,
                    leading_then,
                    starting_with_controller,
                }) = input.next()
                else {
                    unreachable!("source-sentence lookahead changed")
                };
                source_sentences.push((effects, leading_then, starting_with_controller));
            }
            effects.push(EffectAst::DocumentProgram(Box::new(
                self.document_program(source_sentences)?,
            )));
        }
        Ok(())
    }

    fn document_program(
        &mut self,
        source_sentences: Vec<(Vec<EffectAst>, bool, bool)>,
    ) -> Result<CompilerDocumentProgramAst, SymbolResolutionError> {
        let parent_scope = self.context.enter_scope(SymbolScopeKind::Document)?;
        let scope = self.context.current_scope();
        let mut statements = Vec::with_capacity(source_sentences.len());
        for (index, (effects, leading_then, starting_with_controller)) in
            source_sentences.into_iter().enumerate()
        {
            let statement_parent = match self.context.enter_scope(SymbolScopeKind::Line) {
                Ok(parent) => parent,
                Err(error) => {
                    self.context.restore_scope(parent_scope);
                    return Err(error);
                }
            };
            let statement_scope = self.context.current_scope();
            self.context.restore_scope(statement_parent);
            let inventory = reference_inventory(&effects);
            statements.push(CompilerDocumentStatementAst {
                id: CompilerStatementId(u32::try_from(index).unwrap_or(u32::MAX)),
                scope: statement_scope,
                parent_scope: scope,
                effects,
                imports: inventory.imports,
                exports: inventory.exports,
                leading_then,
                starting_with_controller,
            });
        }
        self.context.restore_scope(parent_scope);
        let edges = document_edges(&statements);
        Ok(CompilerDocumentProgramAst {
            scope,
            parent_scope,
            statements,
            edges,
        })
    }
}

#[derive(Default)]
struct ReferenceInventory {
    imports: Vec<SymbolReference>,
    exports: Vec<SymbolReference>,
}

impl SemanticVisitor for ReferenceInventory {
    type Break = ();

    fn visit_reference(&mut self, reference: &SymbolReference) -> ControlFlow<Self::Break> {
        push_unique(&mut self.imports, *reference);
        ControlFlow::Continue(())
    }

    fn visit_reference_binding(
        &mut self,
        reference: &SymbolReference,
    ) -> ControlFlow<Self::Break> {
        push_unique(&mut self.exports, *reference);
        ControlFlow::Continue(())
    }
}

fn reference_inventory(effects: &[EffectAst]) -> ReferenceInventory {
    let mut inventory = ReferenceInventory::default();
    for effect in effects {
        let _ = visit_effect_tree(&mut inventory, effect);
    }
    inventory
        .imports
        .retain(|reference| !inventory.exports.contains(reference));
    inventory
}

fn document_edges(
    statements: &[CompilerDocumentStatementAst],
) -> Vec<CompilerStatementEdgeAst> {
    let mut edges = Vec::new();
    for window in statements.windows(2) {
        let from = &window[0];
        let to = &window[1];
        edges.push(CompilerStatementEdgeAst {
            from: from.id,
            to: to.id,
            kind: if to.leading_then {
                CompilerStatementEdgeKindAst::Then
            } else {
                CompilerStatementEdgeKindAst::Ordered
            },
            references: Vec::new(),
        });
    }
    for (to_index, to) in statements.iter().enumerate() {
        for from in &statements[..to_index] {
            let mut ordinary = Vec::new();
            let mut results = Vec::new();
            for reference in &to.imports {
                if from.exports.contains(reference) {
                    if reference.domain == ObjectDomain::EffectResult {
                        push_unique(&mut results, *reference);
                    } else {
                        push_unique(&mut ordinary, *reference);
                    }
                }
            }
            if !ordinary.is_empty() {
                edges.push(CompilerStatementEdgeAst {
                    from: from.id,
                    to: to.id,
                    kind: CompilerStatementEdgeKindAst::Reference,
                    references: ordinary,
                });
            }
            if !results.is_empty() {
                edges.push(CompilerStatementEdgeAst {
                    from: from.id,
                    to: to.id,
                    kind: CompilerStatementEdgeKindAst::Result,
                    references: results,
                });
            }
        }
    }
    edges
}

fn push_unique(references: &mut Vec<SymbolReference>, reference: SymbolReference) {
    if !references.contains(&reference) {
        references.push(reference);
    }
}

fn common_permission_clause(permission: CompilerPermissionClauseAst) -> CompilerClauseAst {
    let actor = match &permission.actor {
        CompilerPermissionActorAst::Actor(actor) => actor.clone(),
        CompilerPermissionActorAst::Players(_) => ClauseActorAst::SourceController,
    };
    let object = clause_object(&permission.object);
    let verb = match permission.action {
        CompilerCastingActionAst::PlayLand | CompilerCastingActionAst::CastOrPlay => {
            ClauseVerbAst::Play
        }
        CompilerCastingActionAst::CastSpell | CompilerCastingActionAst::ModifySpellCost => {
            ClauseVerbAst::Cast
        }
        CompilerCastingActionAst::ActivateAbility => ClauseVerbAst::Activate,
    };
    let polarity = match permission.disposition {
        CompilerPermissionDispositionAst::Prohibit
        | CompilerPermissionDispositionAst::Restrict => ClausePolarityAst::Negative,
        CompilerPermissionDispositionAst::Execute
        | CompilerPermissionDispositionAst::Permit => ClausePolarityAst::Positive,
    };
    CompilerClauseAst {
        actor: actor.clone(),
        subject: ClauseSubjectAst::Actor(actor),
        action: ClauseActionAst { verb, polarity },
        object: Some(object),
        quantity: None,
        destination: None,
        duration: None,
        condition: None,
        bindings: Vec::new(),
        complements: Vec::new(),
        library: None,
        object_action: None,
        interaction: None,
        resource_choice: None,
        permission: Some(permission),
        provenance: None,
    }
}

fn tagged_permission(
    player: crate::model::parse_types::PlayerAst,
    allow_land: bool,
    object: CompilerObjectOperandAst,
    reference: SymbolReference,
    origin: CompilerCastingOriginAst,
    cost: CompilerCastingCostAst,
    expiration: CompilerPermissionExpirationAst,
    frequency: CompilerPermissionFrequencyAst,
    lands_enter_tapped: bool,
) -> CompilerPermissionClauseAst {
    CompilerPermissionClauseAst {
        disposition: CompilerPermissionDispositionAst::Permit,
        action: casting_action(allow_land),
        actor: compiler_permission_actor(player),
        object,
        qualification: None,
        origin,
        timing: None,
        cost,
        starts: CompilerPermissionStartAst::Immediate,
        expiration,
        frequency,
        linked_object: Some(reference),
        as_copy: false,
        lands_enter_tapped,
    }
}

fn cost_modification_permission(
    actor: ClauseActorAst,
    filter: ObjectFilter,
    adjustment: CompilerCostAdjustmentAst,
    expiration: CompilerPermissionExpirationAst,
    frequency: CompilerPermissionFrequencyAst,
) -> CompilerPermissionClauseAst {
    CompilerPermissionClauseAst {
        disposition: CompilerPermissionDispositionAst::Permit,
        action: CompilerCastingActionAst::ModifySpellCost,
        actor: CompilerPermissionActorAst::Actor(actor),
        object: spell_filter(filter),
        qualification: None,
        origin: CompilerCastingOriginAst::Default,
        timing: None,
        cost: CompilerCastingCostAst {
            payment: CompilerCastingPaymentAst::PrintedCost,
            adjustments: vec![adjustment],
            mana_flexibility: CompilerManaFlexibilityAst::AsWritten,
        },
        starts: CompilerPermissionStartAst::Immediate,
        expiration,
        frequency,
        linked_object: None,
        as_copy: false,
        lands_enter_tapped: false,
    }
}

fn casting_action(allow_land: bool) -> CompilerCastingActionAst {
    if allow_land {
        CompilerCastingActionAst::CastOrPlay
    } else {
        CompilerCastingActionAst::CastSpell
    }
}

fn compiler_permission_actor(
    player: crate::model::parse_types::PlayerAst,
) -> CompilerPermissionActorAst {
    CompilerPermissionActorAst::Actor(super::library_clause_migration::compiler_actor(player))
}

fn spell_filter(filter: ObjectFilter) -> CompilerObjectOperandAst {
    CompilerObjectOperandAst::Filter(CompilerFilterAst::Spell(filter))
}

fn card_filter(filter: ObjectFilter) -> CompilerObjectOperandAst {
    CompilerObjectOperandAst::Filter(CompilerFilterAst::Card(filter))
}

fn object_filter(filter: ObjectFilter) -> CompilerObjectOperandAst {
    CompilerObjectOperandAst::Filter(CompilerFilterAst::Object(filter))
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

fn printed_cost(
    mode: ironsmith_core::value_model::ManaSpendMode,
) -> CompilerCastingCostAst {
    CompilerCastingCostAst {
        payment: CompilerCastingPaymentAst::PrintedCost,
        adjustments: Vec::new(),
        mana_flexibility: mana_flexibility(mode),
    }
}

fn casting_adjustments(
    additional: Option<&crate::mana::ManaCost>,
    reduction: Option<&crate::mana::ManaCost>,
) -> Vec<CompilerCostAdjustmentAst> {
    let mut adjustments = Vec::new();
    if let Some(additional) = additional {
        adjustments.push(CompilerCostAdjustmentAst::AddMana(additional.clone()));
    }
    if let Some(reduction) = reduction {
        adjustments.push(CompilerCostAdjustmentAst::ReduceMana(reduction.clone()));
    }
    adjustments
}

fn mana_flexibility(
    mode: ironsmith_core::value_model::ManaSpendMode,
) -> CompilerManaFlexibilityAst {
    match mode {
        ironsmith_core::value_model::ManaSpendMode::Normal => {
            CompilerManaFlexibilityAst::AsWritten
        }
        ironsmith_core::value_model::ManaSpendMode::AnyColor => {
            CompilerManaFlexibilityAst::AnyColor
        }
        ironsmith_core::value_model::ManaSpendMode::AnyType => {
            CompilerManaFlexibilityAst::AnyType
        }
    }
}

fn compiler_payment(
    payment: &ironsmith_core::MayCastMatchingSpellPayment,
) -> CompilerCastingPaymentAst {
    match payment {
        ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost => {
            CompilerCastingPaymentAst::WithoutPayingManaCost
        }
        ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind) => {
            CompilerCastingPaymentAst::Alternative(compiler_alternative_cast(*kind))
        }
    }
}

fn compiler_alternative_cast(
    kind: crate::filter::AlternativeCastKind,
) -> CompilerAlternativeCastAst {
    match kind {
        crate::filter::AlternativeCastKind::Blitz => CompilerAlternativeCastAst::Blitz,
        crate::filter::AlternativeCastKind::Dash => CompilerAlternativeCastAst::Dash,
        crate::filter::AlternativeCastKind::Flashback => CompilerAlternativeCastAst::Flashback,
        crate::filter::AlternativeCastKind::JumpStart => CompilerAlternativeCastAst::JumpStart,
        crate::filter::AlternativeCastKind::Escape => CompilerAlternativeCastAst::Escape,
        crate::filter::AlternativeCastKind::Madness => CompilerAlternativeCastAst::Madness,
        crate::filter::AlternativeCastKind::Miracle => CompilerAlternativeCastAst::Miracle,
        crate::filter::AlternativeCastKind::Suspend => CompilerAlternativeCastAst::Suspend,
    }
}

fn permission_frequency(maximum: Option<u32>) -> CompilerPermissionFrequencyAst {
    match maximum {
        None => CompilerPermissionFrequencyAst::Unbounded,
        Some(1) => CompilerPermissionFrequencyAst::Once,
        Some(maximum) => CompilerPermissionFrequencyAst::AtMost(maximum),
    }
}

fn bounded_expiration(
    expirations: Vec<CompilerPermissionExpirationAst>,
) -> CompilerPermissionExpirationAst {
    match expirations.as_slice() {
        [expiration] => expiration.clone(),
        _ => CompilerPermissionExpirationAst::BoundedBy(expirations),
    }
}

fn permission_expiration(duration: &Until) -> Option<CompilerPermissionExpirationAst> {
    match duration {
        Until::Forever => Some(CompilerPermissionExpirationAst::Permanent),
        Until::EndOfTurn => Some(CompilerPermissionExpirationAst::UntilEndOfTurn),
        Until::YourNextTurn => Some(CompilerPermissionExpirationAst::UntilYourNextTurn),
        Until::YourNextTurnEnd => {
            Some(CompilerPermissionExpirationAst::UntilYourNextEndStep)
        }
        Until::YourNextUpkeep => Some(CompilerPermissionExpirationAst::UntilYourNextUpkeep),
        Until::ControllersNextUntapStep => {
            Some(CompilerPermissionExpirationAst::UntilControllerNextUntap)
        }
        Until::EndOfCombat => Some(CompilerPermissionExpirationAst::UntilEndOfCombat),
        Until::ThisLeavesTheBattlefield => {
            Some(CompilerPermissionExpirationAst::UntilSourceLeavesBattlefield)
        }
        Until::SourceUntaps => Some(CompilerPermissionExpirationAst::UntilSourceUntaps),
        Until::YouStopControllingThis => {
            Some(CompilerPermissionExpirationAst::WhileYouControlSource)
        }
        Until::TurnsPass(value) => Some(CompilerPermissionExpirationAst::ForTurns(
            CompilerValueAst::Dynamic(value.clone()),
        )),
        Until::EndOfTurnOrAnyPlayerRolls { .. } | Until::ForAsLongAs(_) => None,
    }
}

pub(crate) fn migrate_permission_programs(
    items: &mut [ParsedCardItem],
    context: &mut SemanticMigrationContext<'_>,
) -> Result<(), SymbolResolutionError> {
    let mut migration = PermissionProgramMigration { context };
    for item in items {
        migrate_item(item, &mut migration)?;
    }
    Ok(())
}

fn migrate_item(
    item: &mut ParsedCardItem,
    migration: &mut PermissionProgramMigration<'_, '_>,
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
    migration: &mut PermissionProgramMigration<'_, '_>,
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
    migration: &mut PermissionProgramMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    migration.migrate_effects(&mut modal.header.prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_suffix_effects_ast)?;
    for mode in &mut modal.modes {
        migration.migrate_effects(&mut mode.effects_ast)?;
    }
    Ok(())
}
