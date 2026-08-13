//! Finite PR-28 adapter for resources, choices, votes, and iteration.

use crate::effect::Value;
use crate::model::ast::{EffectAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActionAst, ClauseActorAst, ClauseObjectAst, ClausePolarityAst, ClauseSubjectAst,
    ClauseVerbAst, CompilerClauseAst,
};
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::resource_choice_clauses::{
    CompilerAggregateConstraintAst, CompilerAggregateMetricAst, CompilerChoiceClauseAst,
    CompilerChoiceDomainAst, CompilerChoiceVisibilityAst, CompilerIterationAst,
    CompilerIterationSourceAst, CompilerManaResourceAst, CompilerManaTypeSourceAst,
    CompilerRepetitionKindAst, CompilerResourceAmountAst, CompilerResourceChoiceClauseAst,
    CompilerResourceClauseAst, CompilerResourceKindAst, CompilerResourceOperationAst,
    CompilerVoteAst, CompilerVoteOrderAst,
};
use crate::model::selections::{CompilerValueAst, SelectionCardinalityAst};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolResolutionError, SymbolScopeKind,
};
use crate::model::visit::for_each_nested_effect_vec_mut;
use crate::runtime_backend::front_end::semantic_migration_context::SemanticMigrationContext;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

struct ResourceChoiceMigration<'migration, 'symbols> {
    context: &'migration mut SemanticMigrationContext<'symbols>,
}

impl<'migration, 'symbols> ResourceChoiceMigration<'migration, 'symbols> {
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
        if is_structural_resource_choice(effect) {
            let owned = std::mem::replace(effect, EffectAst::SolveCase);
            *effect = self.migrate_structural(owned)?;
            return Ok(());
        }
        if let Some(clause) = self.resource_choice_clause(effect)? {
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

    fn migrate_structural(
        &mut self,
        effect: EffectAst,
    ) -> Result<EffectAst, SymbolResolutionError> {
        match effect {
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                player,
                tag,
            } => self.object_choice(
                filter,
                count,
                count_value,
                player,
                tag,
                Vec::new(),
                false,
                false,
                None,
            ),
            EffectAst::ChooseObjectsWithAggregateConstraint {
                filter,
                count,
                player,
                tag,
                constraint,
            } => self.object_choice(
                filter,
                count,
                None,
                player,
                tag,
                Vec::new(),
                false,
                false,
                Some(constraint),
            ),
            EffectAst::ChooseObjectsBottomOfLibrary {
                filter,
                count,
                count_value,
                player,
                tag,
            } => self.object_choice(
                filter,
                count,
                count_value,
                player,
                tag,
                vec![crate::zone::Zone::Library],
                false,
                true,
                None,
            ),
            EffectAst::ChooseObjectsTopOfLibrary {
                filter,
                count,
                count_value,
                player,
                tag,
            } => self.object_choice(
                filter,
                count,
                count_value,
                player,
                tag,
                vec![crate::zone::Zone::Library],
                true,
                false,
                None,
            ),
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count,
                player,
                tag,
                zone,
            } => self.object_choice(
                filter,
                count,
                None,
                player,
                tag,
                vec![zone],
                false,
                false,
                None,
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                player,
                tag,
                zones,
                ..
            } => self.object_choice(
                filter,
                count,
                count_value,
                player,
                tag,
                zones,
                false,
                false,
                None,
            ),
            EffectAst::RepeatEffects { count, effects } => self.iteration(
                CompilerRepetitionKindAst::Exactly,
                CompilerIterationSourceAst::Count(CompilerValueAst::Dynamic(count)),
                effects,
            ),
            EffectAst::ForEachOpponent { effects } => self.iteration(
                CompilerRepetitionKindAst::ForEach,
                CompilerIterationSourceAst::Opponents,
                effects,
            ),
            EffectAst::ForEachPlayersFiltered { filter, effects } => self.iteration(
                CompilerRepetitionKindAst::ForEach,
                CompilerIterationSourceAst::Players(filter),
                effects,
            ),
            EffectAst::ForEachPlayer { effects } => self.iteration(
                CompilerRepetitionKindAst::ForEach,
                CompilerIterationSourceAst::Players(PlayerFilter::Any),
                effects,
            ),
            EffectAst::ForEachTargetPlayers {
                count,
                filter,
                effects,
            } => {
                let collection = self.context.bind_selection(
                    ReferenceRole::Target,
                    ObjectDomain::Player,
                    choice_cardinality(count),
                )?;
                self.iteration_with_cardinality(
                    CompilerRepetitionKindAst::ForEach,
                    CompilerIterationSourceAst::SelectedPlayers { filter, collection },
                    Some(choice_cardinality_ast(count, None)),
                    effects,
                )
            }
            EffectAst::ForEachObject { filter, effects } => self.iteration(
                CompilerRepetitionKindAst::ForEach,
                CompilerIterationSourceAst::Objects(filter),
                effects,
            ),
            EffectAst::ForEachTagged { tag, effects } => {
                let reference = if let Some(reference) = self.context.object_reference(&tag) {
                    reference
                } else {
                    self.context.bind_object(
                        Some(tag),
                        ReferenceRole::Affected,
                        Cardinality::Any,
                    )?
                };
                self.iteration(
                    CompilerRepetitionKindAst::ForEach,
                    CompilerIterationSourceAst::Reference(reference),
                    effects,
                )
            }
            EffectAst::ForEachTaggedPlayer { tag, effects } => {
                let reference = if let Some(reference) = self.context.object_reference(&tag) {
                    reference
                } else {
                    self.context.bind_tagged(
                        Some(tag),
                        ReferenceRole::Affected,
                        Cardinality::Any,
                        ObjectDomain::Player,
                    )?
                };
                self.iteration(
                    CompilerRepetitionKindAst::ForEach,
                    CompilerIterationSourceAst::Reference(reference),
                    effects,
                )
            }
            EffectAst::VoteStart {
                options,
                secret,
                starting_with_controller,
            } => self.vote(
                PlayerFilter::Any,
                false,
                CompilerChoiceDomainAst::Named(options),
                secret,
                starting_with_controller,
            ),
            EffectAst::VoteStartObjects {
                filter,
                count,
                secret,
                starting_with_controller,
            } => self.vote_with_cardinality(
                PlayerFilter::Any,
                false,
                CompilerChoiceDomainAst::Object(CompilerObjectOperandAst::Filter(
                    crate::model::selections::CompilerFilterAst::Object(filter),
                )),
                secret,
                starting_with_controller,
                choice_cardinality(count),
            ),
            EffectAst::VoteStartPlayers {
                filter,
                exclude_voter,
                secret,
                starting_with_controller,
            } => self.vote(
                PlayerFilter::Any,
                exclude_voter,
                CompilerChoiceDomainAst::Player {
                    filter,
                    exclude_previous: 0,
                },
                secret,
                starting_with_controller,
            ),
            effect => Ok(effect),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn object_choice(
        &mut self,
        filter: ObjectFilter,
        count: crate::effect::ChoiceCount,
        count_value: Option<Value>,
        player: crate::model::parse_types::PlayerAst,
        tag: TagKey,
        zones: Vec<crate::zone::Zone>,
        top_only: bool,
        bottom_only: bool,
        aggregate: Option<crate::effect::ChoiceAggregateConstraint>,
    ) -> Result<EffectAst, SymbolResolutionError> {
        let reference_cardinality = choice_cardinality(count);
        let chosen = self.context.bind_tagged(
            Some(tag),
            ReferenceRole::Chosen,
            reference_cardinality,
            ObjectDomain::Object,
        )?;
        let chooser = super::library_clause_migration::compiler_actor(player);
        let scope = self.context.current_scope();
        Ok(EffectAst::Clause(common_clause(
            chooser.clone(),
            ClauseVerbAst::Choose,
            None,
            CompilerResourceChoiceClauseAst::Choice(CompilerChoiceClauseAst {
                chooser,
                visibility: CompilerChoiceVisibilityAst::Public,
                domain: CompilerChoiceDomainAst::Object(CompilerObjectOperandAst::Filter(
                    crate::model::selections::CompilerFilterAst::Object(filter),
                )),
                cardinality: choice_cardinality_ast(count, count_value),
                random: count.random,
                zones,
                top_only,
                bottom_only,
                aggregate: aggregate.map(|aggregate| CompilerAggregateConstraintAst {
                    metric: match aggregate.metric {
                        crate::effect::ChoiceAggregateMetric::Power => {
                            CompilerAggregateMetricAst::Power
                        }
                        crate::effect::ChoiceAggregateMetric::Toughness => {
                            CompilerAggregateMetricAst::Toughness
                        }
                        crate::effect::ChoiceAggregateMetric::ManaValue => {
                            CompilerAggregateMetricAst::ManaValue
                        }
                    },
                    minimum: aggregate.minimum.map(CompilerValueAst::Dynamic),
                    maximum: CompilerValueAst::Dynamic(aggregate.maximum),
                }),
                scope,
                chosen,
            }),
        )))
    }

    fn iteration(
        &mut self,
        kind: CompilerRepetitionKindAst,
        source: CompilerIterationSourceAst,
        body: Vec<EffectAst>,
    ) -> Result<EffectAst, SymbolResolutionError> {
        self.iteration_with_cardinality(kind, source, None, body)
    }

    fn iteration_with_cardinality(
        &mut self,
        kind: CompilerRepetitionKindAst,
        source: CompilerIterationSourceAst,
        selection_cardinality: Option<SelectionCardinalityAst>,
        mut body: Vec<EffectAst>,
    ) -> Result<EffectAst, SymbolResolutionError> {
        let domain = iteration_domain(&source);
        let parent_scope = self.context.enter_scope(SymbolScopeKind::Iteration)?;
        let scope = self.context.current_scope();
        let iterator = self.context.bind_selection(
            ReferenceRole::Iteration,
            domain,
            Cardinality::ExactlyOne,
        )?;
        self.context
            .remember_reference(TagKey::from(crate::host::IT_TAG), iterator);
        let body_result = self.migrate_effects(&mut body);
        self.context.restore_scope(parent_scope);
        body_result?;
        let aggregate = self.context.bind_selection(
            ReferenceRole::Affected,
            ObjectDomain::Value,
            Cardinality::ExactlyOne,
        )?;
        Ok(EffectAst::Iteration(Box::new(CompilerIterationAst {
            kind,
            source,
            parent_scope,
            scope,
            iterator,
            selection_cardinality,
            body,
            aggregate: Some(aggregate),
        })))
    }

    fn vote(
        &mut self,
        voters: PlayerFilter,
        exclude_voter: bool,
        options: CompilerChoiceDomainAst,
        secret: bool,
        starts_with_controller: bool,
    ) -> Result<EffectAst, SymbolResolutionError> {
        self.vote_with_cardinality(
            voters,
            exclude_voter,
            options,
            secret,
            starts_with_controller,
            Cardinality::ExactlyOne,
        )
    }

    fn vote_with_cardinality(
        &mut self,
        voters: PlayerFilter,
        exclude_voter: bool,
        options: CompilerChoiceDomainAst,
        secret: bool,
        starts_with_controller: bool,
        cardinality: Cardinality,
    ) -> Result<EffectAst, SymbolResolutionError> {
        let parent = self.context.enter_scope(SymbolScopeKind::Branch)?;
        let choice_scope = self.context.current_scope();
        let choices = self.context.bind_selection(
            ReferenceRole::Chosen,
            choice_domain(&options),
            Cardinality::Any,
        );
        self.context.restore_scope(parent);
        let choices = choices?;
        let tally = self.context.bind_selection(
            ReferenceRole::Affected,
            ObjectDomain::Value,
            Cardinality::Any,
        )?;
        Ok(EffectAst::Vote(CompilerVoteAst {
            voters,
            exclude_voter,
            visibility: if secret {
                CompilerChoiceVisibilityAst::Secret
            } else {
                CompilerChoiceVisibilityAst::Public
            },
            order: CompilerVoteOrderAst::TurnOrder,
            starts_with_controller,
            options,
            votes_per_voter: cardinality_ast(cardinality),
            choice_scope,
            choices,
            tally,
        }))
    }

    fn resource_choice_clause(
        &mut self,
        effect: &EffectAst,
    ) -> Result<Option<CompilerClauseAst>, SymbolResolutionError> {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return Ok(None);
        };
        let owner = super::library_clause_migration::compiler_actor(subject_verb.subject.player);
        let clause = match &subject_verb.action {
            SubjectVerbActionAst::Draw { count } => self.resource(
                owner,
                ClauseVerbAst::Draw,
                CompilerResourceOperationAst::Draw,
                CompilerResourceKindAst::Cards,
                value_amount(count),
                None,
                false,
                None,
                ReferenceRole::Affected,
                ObjectDomain::Card,
            )?,
            SubjectVerbActionAst::LoseLife { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Lose,
                CompilerResourceOperationAst::Lose,
                CompilerResourceKindAst::Life,
                value_amount(amount),
            )?,
            SubjectVerbActionAst::PayLife { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Pay,
                CompilerResourceOperationAst::Pay,
                CompilerResourceKindAst::Life,
                value_amount(amount),
            )?,
            SubjectVerbActionAst::GainLife { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Gain,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Life,
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddMana { mana } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::Fixed(mana.clone())),
                CompilerResourceAmountAst::Value(CompilerValueAst::Fixed(fixed_len(mana.len()))),
            )?,
            SubjectVerbActionAst::AddManaScaled { mana, amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::Fixed(mana.clone())),
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::AnyColor {
                    available: available_colors.clone(),
                    distinct: *distinct_colors,
                }),
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddManaAnyOneColor { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::AnyOneColor),
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddManaChosenColor {
                amount,
                fixed_option,
            } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::ChosenColor(*fixed_option)),
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            } => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::LandCouldProduce {
                    filter: land_filter.clone(),
                    allow_colorless: *allow_colorless,
                    same_type: *same_type,
                    source: match mana_type_source {
                        crate::effects::ManaTypeSource::MatchingLandsCouldProduce => {
                            CompilerManaTypeSourceAst::MatchingLandsCouldProduce
                        }
                        crate::effects::ManaTypeSource::TriggeringEventProduced => {
                            CompilerManaTypeSourceAst::TriggeringEventProduced
                        }
                    },
                }),
                value_amount(amount),
            )?,
            SubjectVerbActionAst::AddManaColorsAmong { filter }
            | SubjectVerbActionAst::AddOneManaAnyColorAmong { filter, .. } => self
                .player_value_resource(
                    owner,
                    ClauseVerbAst::Add,
                    CompilerResourceOperationAst::Gain,
                    CompilerResourceKindAst::Mana(CompilerManaResourceAst::ColorsAmong(
                        filter.clone(),
                    )),
                    CompilerResourceAmountAst::Value(CompilerValueAst::Fixed(1)),
                )?,
            SubjectVerbActionAst::AddManaCommanderIdentity { amount } => self
                .player_value_resource(
                    owner,
                    ClauseVerbAst::Add,
                    CompilerResourceOperationAst::Gain,
                    CompilerResourceKindAst::Mana(CompilerManaResourceAst::CommanderIdentity),
                    value_amount(amount),
                )?,
            SubjectVerbActionAst::EnergyCounters { count } => self.player_value_resource(
                owner,
                ClauseVerbAst::Gain,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Energy,
                value_amount(count),
            )?,
            SubjectVerbActionAst::ExperienceCounters { count } => self.player_value_resource(
                owner,
                ClauseVerbAst::Gain,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Experience,
                value_amount(count),
            )?,
            SubjectVerbActionAst::TicketCounters { count } => self.player_value_resource(
                owner,
                ClauseVerbAst::Gain,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Ticket,
                value_amount(count),
            )?,
            SubjectVerbActionAst::PoisonCounters { count } => self.player_value_resource(
                owner,
                ClauseVerbAst::Gain,
                CompilerResourceOperationAst::Gain,
                CompilerResourceKindAst::Poison,
                value_amount(count),
            )?,
            SubjectVerbActionAst::PayEnergy { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Pay,
                CompilerResourceOperationAst::Pay,
                CompilerResourceKindAst::Energy,
                value_amount(amount),
            )?,
            SubjectVerbActionAst::PayAnyEnergy { min_amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Pay,
                CompilerResourceOperationAst::Pay,
                CompilerResourceKindAst::Energy,
                any_amount(*min_amount),
            )?,
            SubjectVerbActionAst::PayAnyLife { min_amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Pay,
                CompilerResourceOperationAst::Pay,
                CompilerResourceKindAst::Life,
                any_amount(*min_amount),
            )?,
            SubjectVerbActionAst::PayMana {
                cost,
                x_value,
                x_maximum,
            } => self.player_value_resource(
                owner,
                ClauseVerbAst::Pay,
                CompilerResourceOperationAst::Pay,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::Cost {
                    cost: cost.clone(),
                    x_value: x_value.clone().map(CompilerValueAst::Dynamic),
                    x_maximum: x_maximum.clone().map(CompilerValueAst::Dynamic),
                }),
                CompilerResourceAmountAst::Value(CompilerValueAst::Fixed(1)),
            )?,
            SubjectVerbActionAst::DoubleManaPool => self.player_value_resource(
                owner,
                ClauseVerbAst::Add,
                CompilerResourceOperationAst::Double,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::Pool),
                CompilerResourceAmountAst::All,
            )?,
            SubjectVerbActionAst::EmptyManaPool => self.player_value_resource(
                owner,
                ClauseVerbAst::Remove,
                CompilerResourceOperationAst::Empty,
                CompilerResourceKindAst::Mana(CompilerManaResourceAst::Pool),
                CompilerResourceAmountAst::All,
            )?,
            SubjectVerbActionAst::SetLifeTotal { amount } => self.player_value_resource(
                owner,
                ClauseVerbAst::Become,
                CompilerResourceOperationAst::Set,
                CompilerResourceKindAst::Life,
                value_amount(amount),
            )?,
            SubjectVerbActionAst::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            } => self.resource(
                owner,
                ClauseVerbAst::Discard,
                CompilerResourceOperationAst::Discard,
                CompilerResourceKindAst::Cards,
                if *any_number {
                    any_amount(0)
                } else {
                    value_amount(count)
                },
                filter.as_ref().map(|filter| {
                    CompilerObjectOperandAst::Filter(
                        crate::model::selections::CompilerFilterAst::Card(filter.clone()),
                    )
                }),
                *random,
                tag.clone(),
                ReferenceRole::Discarded,
                ObjectDomain::Card,
            )?,
            SubjectVerbActionAst::DiscardHand => self.resource(
                owner,
                ClauseVerbAst::Discard,
                CompilerResourceOperationAst::Discard,
                CompilerResourceKindAst::Cards,
                CompilerResourceAmountAst::All,
                None,
                false,
                None,
                ReferenceRole::Discarded,
                ObjectDomain::Card,
            )?,
            SubjectVerbActionAst::Tap { target } => {
                let objects = self.context.target_operand(target)?;
                self.object_state_resource(
                    owner,
                    ClauseVerbAst::Tap,
                    CompilerResourceOperationAst::Tap,
                    objects,
                )?
            }
            SubjectVerbActionAst::Untap { target } => {
                let objects = self.context.target_operand(target)?;
                self.object_state_resource(
                    owner,
                    ClauseVerbAst::Untap,
                    CompilerResourceOperationAst::Untap,
                    objects,
                )?
            }
            SubjectVerbActionAst::TapAll { filter } => self.object_state_resource(
                owner,
                ClauseVerbAst::Tap,
                CompilerResourceOperationAst::Tap,
                CompilerObjectOperandAst::Filter(
                    crate::model::selections::CompilerFilterAst::Object(filter.clone()),
                ),
            )?,
            SubjectVerbActionAst::UntapAll { filter } => self.object_state_resource(
                owner,
                ClauseVerbAst::Untap,
                CompilerResourceOperationAst::Untap,
                CompilerObjectOperandAst::Filter(
                    crate::model::selections::CompilerFilterAst::Object(filter.clone()),
                ),
            )?,
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
                ..
            } => {
                let objects = target
                    .as_ref()
                    .map(|target| self.context.target_operand(target))
                    .transpose()?
                    .unwrap_or_else(|| {
                        CompilerObjectOperandAst::Filter(
                            crate::model::selections::CompilerFilterAst::Object(filter.clone()),
                        )
                    });
                self.resource(
                    owner,
                    ClauseVerbAst::Sacrifice,
                    CompilerResourceOperationAst::Sacrifice,
                    CompilerResourceKindAst::ObjectState,
                    CompilerResourceAmountAst::Value(CompilerValueAst::Fixed(fixed_u32(*count))),
                    Some(objects),
                    false,
                    None,
                    ReferenceRole::Sacrificed,
                    ObjectDomain::Card,
                )?
            }
            SubjectVerbActionAst::SacrificeAll { filter } => self.resource(
                owner,
                ClauseVerbAst::Sacrifice,
                CompilerResourceOperationAst::Sacrifice,
                CompilerResourceKindAst::ObjectState,
                CompilerResourceAmountAst::All,
                Some(CompilerObjectOperandAst::Filter(
                    crate::model::selections::CompilerFilterAst::Object(filter.clone()),
                )),
                false,
                None,
                ReferenceRole::Sacrificed,
                ObjectDomain::Card,
            )?,
            SubjectVerbActionAst::ChooseColor => self.choice(
                owner,
                CompilerChoiceDomainAst::Color,
                false,
                None,
                ObjectDomain::Value,
            )?,
            SubjectVerbActionAst::ChooseCardType { options } => self.choice(
                owner,
                CompilerChoiceDomainAst::CardType(options.clone()),
                false,
                None,
                ObjectDomain::Value,
            )?,
            SubjectVerbActionAst::ChooseNamedOption { options } => self.choice(
                owner,
                CompilerChoiceDomainAst::Named(options.clone()),
                false,
                None,
                ObjectDomain::Value,
            )?,
            SubjectVerbActionAst::ChooseCreatureType {
                excluded_subtypes,
                family,
            } => self.choice(
                owner,
                CompilerChoiceDomainAst::CreatureType {
                    excluded: excluded_subtypes.clone(),
                    family: *family,
                },
                false,
                None,
                ObjectDomain::Value,
            )?,
            SubjectVerbActionAst::ChooseLandType { exclude_basic } => self.choice(
                owner,
                CompilerChoiceDomainAst::LandType {
                    exclude_basic: *exclude_basic,
                },
                false,
                None,
                ObjectDomain::Value,
            )?,
            SubjectVerbActionAst::ChooseCardName { filter, tag } => self.choice(
                owner,
                CompilerChoiceDomainAst::CardName(filter.clone()),
                false,
                Some(tag.clone()),
                ObjectDomain::Card,
            )?,
            SubjectVerbActionAst::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            } => self.choice(
                owner,
                CompilerChoiceDomainAst::Player {
                    filter: filter.clone(),
                    exclude_previous: *exclude_previous_choices,
                },
                *random,
                Some(tag.clone()),
                ObjectDomain::Player,
            )?,
            _ => return Ok(None),
        };
        Ok(Some(clause))
    }

    fn player_value_resource(
        &mut self,
        owner: ClauseActorAst,
        verb: ClauseVerbAst,
        operation: CompilerResourceOperationAst,
        resource: CompilerResourceKindAst,
        amount: CompilerResourceAmountAst,
    ) -> Result<CompilerClauseAst, SymbolResolutionError> {
        self.resource(
            owner,
            verb,
            operation,
            resource,
            amount,
            None,
            false,
            None,
            ReferenceRole::Affected,
            ObjectDomain::Value,
        )
    }

    fn object_state_resource(
        &mut self,
        owner: ClauseActorAst,
        verb: ClauseVerbAst,
        operation: CompilerResourceOperationAst,
        objects: CompilerObjectOperandAst,
    ) -> Result<CompilerClauseAst, SymbolResolutionError> {
        self.resource(
            owner,
            verb,
            operation,
            CompilerResourceKindAst::ObjectState,
            CompilerResourceAmountAst::All,
            Some(objects),
            false,
            None,
            ReferenceRole::Affected,
            ObjectDomain::Card,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resource(
        &mut self,
        owner: ClauseActorAst,
        verb: ClauseVerbAst,
        operation: CompilerResourceOperationAst,
        resource: CompilerResourceKindAst,
        amount: CompilerResourceAmountAst,
        objects: Option<CompilerObjectOperandAst>,
        random: bool,
        tag: Option<TagKey>,
        role: ReferenceRole,
        domain: ObjectDomain,
    ) -> Result<CompilerClauseAst, SymbolResolutionError> {
        let result_cardinality = if domain == ObjectDomain::Value {
            Cardinality::ExactlyOne
        } else {
            Cardinality::Any
        };
        let result = self
            .context
            .bind_tagged(tag, role, result_cardinality, domain)?;
        let clause_object = objects.as_ref().map(clause_object);
        Ok(common_clause(
            owner.clone(),
            verb,
            clause_object,
            CompilerResourceChoiceClauseAst::Resource(CompilerResourceClauseAst {
                operation,
                owner,
                resource,
                amount,
                objects,
                random,
                result,
            }),
        ))
    }

    fn choice(
        &mut self,
        chooser: ClauseActorAst,
        domain: CompilerChoiceDomainAst,
        random: bool,
        tag: Option<TagKey>,
        symbol_domain: ObjectDomain,
    ) -> Result<CompilerClauseAst, SymbolResolutionError> {
        let chosen = self.context.bind_tagged(
            tag,
            ReferenceRole::Chosen,
            Cardinality::ExactlyOne,
            symbol_domain,
        )?;
        let scope = self.context.current_scope();
        Ok(common_clause(
            chooser.clone(),
            ClauseVerbAst::Choose,
            None,
            CompilerResourceChoiceClauseAst::Choice(CompilerChoiceClauseAst {
                chooser,
                visibility: CompilerChoiceVisibilityAst::Public,
                domain,
                cardinality: cardinality_ast(Cardinality::ExactlyOne),
                random,
                zones: Vec::new(),
                top_only: false,
                bottom_only: false,
                aggregate: None,
                scope,
                chosen,
            }),
        ))
    }
}

pub(crate) fn migrate_resource_choice_clauses(
    items: &mut [ParsedCardItem],
    context: &mut SemanticMigrationContext<'_>,
) -> Result<(), SymbolResolutionError> {
    let mut migration = ResourceChoiceMigration { context };
    for item in items {
        migrate_item(item, &mut migration)?;
    }
    Ok(())
}

fn migrate_item(
    item: &mut ParsedCardItem,
    migration: &mut ResourceChoiceMigration<'_, '_>,
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
    migration: &mut ResourceChoiceMigration<'_, '_>,
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
    migration: &mut ResourceChoiceMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    migration.migrate_effects(&mut modal.header.prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_suffix_effects_ast)?;
    for mode in &mut modal.modes {
        migration.migrate_effects(&mut mode.effects_ast)?;
    }
    Ok(())
}

fn common_clause(
    actor: ClauseActorAst,
    verb: ClauseVerbAst,
    object: Option<ClauseObjectAst>,
    resource_choice: CompilerResourceChoiceClauseAst,
) -> CompilerClauseAst {
    CompilerClauseAst {
        actor: actor.clone(),
        subject: ClauseSubjectAst::Actor(actor),
        action: ClauseActionAst {
            verb,
            polarity: ClausePolarityAst::Positive,
        },
        object,
        quantity: None,
        destination: None,
        duration: None,
        condition: None,
        bindings: Vec::new(),
        complements: Vec::new(),
        library: None,
        object_action: None,
        interaction: None,
        resource_choice: Some(resource_choice),
        permission: None,
        provenance: None,
    }
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

fn is_structural_resource_choice(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::ChooseObjects { .. }
            | EffectAst::ChooseObjectsWithAggregateConstraint { .. }
            | EffectAst::ChooseObjectsBottomOfLibrary { .. }
            | EffectAst::ChooseObjectsTopOfLibrary { .. }
            | EffectAst::ChooseTaggedObjectsInZone { .. }
            | EffectAst::ChooseObjectsAcrossZones { .. }
            | EffectAst::RepeatEffects { .. }
            | EffectAst::ForEachOpponent { .. }
            | EffectAst::ForEachPlayersFiltered { .. }
            | EffectAst::ForEachPlayer { .. }
            | EffectAst::ForEachTargetPlayers { .. }
            | EffectAst::ForEachObject { .. }
            | EffectAst::ForEachTagged { .. }
            | EffectAst::ForEachTaggedPlayer { .. }
            | EffectAst::VoteStart { .. }
            | EffectAst::VoteStartObjects { .. }
            | EffectAst::VoteStartPlayers { .. }
    )
}

fn iteration_domain(source: &CompilerIterationSourceAst) -> ObjectDomain {
    match source {
        CompilerIterationSourceAst::Opponents | CompilerIterationSourceAst::Players(_) => {
            ObjectDomain::Player
        }
        CompilerIterationSourceAst::SelectedPlayers { .. } => ObjectDomain::Player,
        CompilerIterationSourceAst::Objects(_) => ObjectDomain::Object,
        CompilerIterationSourceAst::Reference(reference) => reference.domain,
        CompilerIterationSourceAst::Count(_) => ObjectDomain::Value,
    }
}

fn choice_domain(domain: &CompilerChoiceDomainAst) -> ObjectDomain {
    match domain {
        CompilerChoiceDomainAst::Player { .. } => ObjectDomain::Player,
        CompilerChoiceDomainAst::CardName(_) => ObjectDomain::Card,
        CompilerChoiceDomainAst::Object(object) => match object {
            CompilerObjectOperandAst::Selection(selection) => selection.domain.symbol_domain(),
            CompilerObjectOperandAst::Reference(reference) => reference.domain,
            CompilerObjectOperandAst::Filter(filter) => filter.domain(),
            CompilerObjectOperandAst::Source => ObjectDomain::Object,
        },
        _ => ObjectDomain::Value,
    }
}

fn value_amount(value: &Value) -> CompilerResourceAmountAst {
    CompilerResourceAmountAst::Value(CompilerValueAst::Dynamic(value.clone()))
}

fn any_amount(minimum: u32) -> CompilerResourceAmountAst {
    CompilerResourceAmountAst::Any {
        minimum: CompilerValueAst::Fixed(fixed_u32(minimum)),
    }
}

fn cardinality_ast(cardinality: Cardinality) -> SelectionCardinalityAst {
    let (minimum, maximum) = match cardinality {
        Cardinality::ExactlyOne => (1, Some(1)),
        Cardinality::ZeroOrOne => (0, Some(1)),
        Cardinality::OneOrMore => (1, None),
        Cardinality::Any => (0, None),
        Cardinality::Fixed(count) => (count, Some(count)),
        Cardinality::Range { min, max } => (min, max),
    };
    SelectionCardinalityAst {
        min: CompilerValueAst::Fixed(fixed_u32(minimum)),
        max: maximum.map(|maximum| CompilerValueAst::Fixed(fixed_u32(maximum))),
        reference_cardinality: cardinality,
    }
}

fn choice_cardinality(count: crate::effect::ChoiceCount) -> Cardinality {
    match (count.min, count.max) {
        (1, Some(1)) if !count.dynamic_x => Cardinality::ExactlyOne,
        (0, Some(1)) if !count.dynamic_x => Cardinality::ZeroOrOne,
        (minimum, maximum) => Cardinality::Range {
            min: u32::try_from(minimum).unwrap_or(u32::MAX),
            max: maximum.map(|maximum| u32::try_from(maximum).unwrap_or(u32::MAX)),
        },
    }
}

fn choice_cardinality_ast(
    count: crate::effect::ChoiceCount,
    count_value: Option<Value>,
) -> SelectionCardinalityAst {
    let reference_cardinality = choice_cardinality(count);
    SelectionCardinalityAst {
        min: if count.dynamic_x {
            CompilerValueAst::X
        } else {
            CompilerValueAst::Fixed(fixed_len(count.min))
        },
        max: count_value.map(CompilerValueAst::Dynamic).or_else(|| {
            count
                .max
                .map(|maximum| CompilerValueAst::Fixed(fixed_len(maximum)))
        }),
        reference_cardinality,
    }
}

fn fixed_len(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn fixed_u32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
