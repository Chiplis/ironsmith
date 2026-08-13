//! Shared state for the finite PR-25 through PR-29 semantic adapters.
//!
//! Legacy tags are admitted only at this boundary. Canonical domain clauses
//! receive typed symbol references and never inspect tag spelling.

use std::collections::HashMap;

use crate::effect::ChoiceCount;
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::parse_types::TargetAst;
use crate::model::selections::{
    CompilerFilterAst, CompilerSelectionAst, CompilerValueAst, SelectionCardinalityAst,
    SelectionDomainAst, SelectionKindAst, SelectionLegalityAst,
};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError,
    SymbolScopeId, SymbolScopeKind, SymbolTable,
};
use crate::tag::TagKey;
use crate::target::ObjectFilter;

pub(crate) struct SemanticMigrationContext<'a> {
    symbols: &'a mut SymbolTable,
    tagged_objects: HashMap<TagKey, Vec<(SymbolScopeId, SymbolReference)>>,
    current_scope: SymbolScopeId,
}

impl<'a> SemanticMigrationContext<'a> {
    pub(crate) fn new(symbols: &'a mut SymbolTable) -> Self {
        let current_scope = symbols.root_scope();
        Self {
            symbols,
            tagged_objects: HashMap::new(),
            current_scope,
        }
    }

    pub(crate) fn current_scope(&self) -> SymbolScopeId {
        self.current_scope
    }

    pub(crate) fn enter_scope(
        &mut self,
        kind: SymbolScopeKind,
    ) -> Result<SymbolScopeId, SymbolResolutionError> {
        let parent = self.current_scope;
        self.current_scope = self.symbols.create_scope(parent, kind)?;
        Ok(parent)
    }

    pub(crate) fn restore_scope(&mut self, scope: SymbolScopeId) {
        self.current_scope = scope;
    }

    pub(crate) fn bind_object(
        &mut self,
        tag: Option<TagKey>,
        role: ReferenceRole,
        cardinality: Cardinality,
    ) -> Result<SymbolReference, SymbolResolutionError> {
        self.bind_tagged(tag, role, cardinality, ObjectDomain::Card)
    }

    pub(crate) fn bind_tagged(
        &mut self,
        tag: Option<TagKey>,
        role: ReferenceRole,
        cardinality: Cardinality,
        domain: ObjectDomain,
    ) -> Result<SymbolReference, SymbolResolutionError> {
        if let Some(reference) = tag.as_ref().and_then(|tag| self.object_reference(tag)) {
            return Ok(reference);
        }
        let reference = SymbolReference {
            symbol: self
                .symbols
                .bind(self.current_scope, role, cardinality, domain, None)?,
            role,
            domain,
            cardinality,
        };
        if let Some(tag) = tag {
            self.tagged_objects
                .entry(tag)
                .or_default()
                .push((self.current_scope, reference));
        }
        Ok(reference)
    }

    pub(crate) fn object_reference(&self, tag: &TagKey) -> Option<SymbolReference> {
        self.tagged_objects.get(tag).and_then(|bindings| {
            bindings
                .iter()
                .rev()
                .find(|(scope, _)| {
                    self.symbols
                        .scope_is_ancestor_of(*scope, self.current_scope)
                })
                .map(|(_, reference)| *reference)
        })
    }

    pub(crate) fn remember_reference(&mut self, tag: TagKey, reference: SymbolReference) {
        self.tagged_objects
            .entry(tag)
            .or_default()
            .push((self.current_scope, reference));
    }

    pub(crate) fn bind_selection(
        &mut self,
        role: ReferenceRole,
        domain: ObjectDomain,
        cardinality: Cardinality,
    ) -> Result<SymbolReference, SymbolResolutionError> {
        Ok(SymbolReference {
            symbol: self
                .symbols
                .bind(self.current_scope, role, cardinality, domain, None)?,
            role,
            domain,
            cardinality,
        })
    }

    pub(crate) fn target_operand(
        &mut self,
        target: &TargetAst,
    ) -> Result<CompilerObjectOperandAst, SymbolResolutionError> {
        self.target_operand_with_count(target, ChoiceCount::exactly(1), None)
    }

    pub(crate) fn counted_target_operand(
        &mut self,
        target: &TargetAst,
        count: ChoiceCount,
        count_value: Option<crate::effect::Value>,
    ) -> Result<CompilerObjectOperandAst, SymbolResolutionError> {
        self.target_operand_with_count(target, count, count_value)
    }

    fn target_operand_with_count(
        &mut self,
        target: &TargetAst,
        count: ChoiceCount,
        count_value: Option<crate::effect::Value>,
    ) -> Result<CompilerObjectOperandAst, SymbolResolutionError> {
        let domain = match target {
            TargetAst::Source(_) => return Ok(CompilerObjectOperandAst::Source),
            TargetAst::Tagged(tag, _) => {
                let reference = if let Some(reference) = self.object_reference(tag) {
                    reference
                } else {
                    self.bind_object(
                        Some(tag.clone()),
                        ReferenceRole::Affected,
                        choice_cardinality(count),
                    )?
                };
                return Ok(CompilerObjectOperandAst::Reference(reference));
            }
            TargetAst::WithCount(target, count) => {
                return self.target_operand_with_count(target, *count, None);
            }
            TargetAst::WithCountValue(target, count, value) => {
                return self.target_operand_with_count(target, *count, Some(value.clone()));
            }
            TargetAst::AnyTarget(_) => SelectionDomainAst::AnyTarget,
            TargetAst::AnyOtherTarget(_) => SelectionDomainAst::AnyOtherTarget,
            TargetAst::ObjectOrPlayer(object, player, _) => SelectionDomainAst::ObjectOrPlayer {
                object: object.clone(),
                player: player.clone(),
            },
            TargetAst::PlayerOrPlaneswalker(player, _) => {
                SelectionDomainAst::PlayerOrPlaneswalker(player.clone())
            }
            TargetAst::AttackedPlayerOrPlaneswalker(_) => {
                SelectionDomainAst::AttackedPlayerOrPlaneswalker
            }
            TargetAst::Spell(_) => SelectionDomainAst::Spell(ObjectFilter::default()),
            TargetAst::Player(player, _) => {
                SelectionDomainAst::Filter(CompilerFilterAst::Player(player.clone()))
            }
            TargetAst::Object(object, _, _) => {
                SelectionDomainAst::Filter(CompilerFilterAst::Object(object.clone()))
            }
        };
        let reference_cardinality = choice_cardinality(count);
        let binding = self.bind_selection(
            ReferenceRole::Target,
            domain.symbol_domain(),
            reference_cardinality,
        )?;
        Ok(CompilerObjectOperandAst::Selection(CompilerSelectionAst {
            kind: SelectionKindAst::Target,
            domain,
            cardinality: SelectionCardinalityAst {
                min: if count.dynamic_x {
                    CompilerValueAst::X
                } else {
                    fixed_usize(count.min)
                },
                max: count_value
                    .map(CompilerValueAst::Dynamic)
                    .or_else(|| count.max.map(fixed_usize)),
                reference_cardinality,
            },
            legality: SelectionLegalityAst {
                targetable: true,
                zones: Vec::new(),
                controller_only: false,
                owner_only: false,
                distinct: true,
                random: count.random,
            },
            binding,
            provenance: None,
        }))
    }
}

fn fixed_usize(value: usize) -> CompilerValueAst {
    CompilerValueAst::Fixed(i32::try_from(value).unwrap_or(i32::MAX))
}

fn choice_cardinality(count: ChoiceCount) -> Cardinality {
    match (count.min, count.max) {
        (1, Some(1)) if !count.dynamic_x => Cardinality::ExactlyOne,
        (0, Some(1)) if !count.dynamic_x => Cardinality::ZeroOrOne,
        (min, max) => Cardinality::Range {
            min: u32::try_from(min).unwrap_or(u32::MAX),
            max: max.map(|max| u32::try_from(max).unwrap_or(u32::MAX)),
        },
    }
}
