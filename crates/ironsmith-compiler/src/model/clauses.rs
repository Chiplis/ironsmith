//! Composable semantic clauses shared by effect recognition and lowering.
//!
//! The legacy effect AST contains many sentence-shaped variants. New grammar
//! should instead assemble these orthogonal clause parts, then lower the
//! resulting `CompilerClauseAst` through the canonical backend boundary.

use crate::effect::ValueComparisonOperator;
use crate::model::costs::CompilerTotalCost;
use crate::model::interaction_clauses::{
    CompilerInteractionClauseAst, CompilerPreventionClauseAst,
};
use crate::model::library_clauses::CompilerLibraryClauseAst;
use crate::model::object_action_clauses::{
    CompilerCreationKindAst, CompilerEntryStateAst, CompilerObjectActionClauseAst,
    CompilerObjectOperandAst,
};
use crate::model::provenance::SemanticProvenance;
use crate::model::selections::{CompilerFilterAst, CompilerSelectionAst, CompilerValueAst};
use crate::model::symbols::SymbolReference;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseVerbAst {
    Add,
    Attach,
    Become,
    Cast,
    Choose,
    Control,
    Copy,
    Counter,
    Create,
    DealDamage,
    Destroy,
    Discard,
    Draw,
    Exchange,
    Exile,
    Fight,
    Gain,
    Give,
    Look,
    Lose,
    Mill,
    Move,
    Pay,
    Play,
    Prevent,
    Put,
    Remove,
    Return,
    Reveal,
    Sacrifice,
    Search,
    Shuffle,
    Tap,
    Transform,
    Untap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClausePolarityAst {
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClauseActionAst {
    pub verb: ClauseVerbAst,
    pub polarity: ClausePolarityAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClauseActorAst {
    SourceController,
    ActivePlayer,
    EachOpponent,
    EachPlayer,
    Selection(CompilerSelectionAst),
    Reference(SymbolReference),
    Player(CompilerPlayerAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerPlayerAst {
    Any,
    Chosen,
    Defending,
    Attacking,
    Opponent,
    Target,
    TargetOpponent,
    Enchanted,
    OtherThanSourceController,
    Contextual,
    TriggeringSourceController,
    ReferencedObjectController,
    ReferencedObjectOwner,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClauseSubjectAst {
    Source,
    Actor(ClauseActorAst),
    Selection(CompilerSelectionAst),
    Filter(CompilerFilterAst),
    Reference(SymbolReference),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClauseObjectAst {
    Subject(ClauseSubjectAst),
    Selection(CompilerSelectionAst),
    Filter(CompilerFilterAst),
    Reference(SymbolReference),
    Cost(CompilerTotalCost),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseQuantityUnitAst {
    Objects,
    Cards,
    Players,
    Life,
    Damage,
    Mana,
    Counters,
    Turns,
    Times,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseDistributionAst {
    Total,
    Each,
    Divided,
    UpTo,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClauseQuantityAst {
    pub value: CompilerValueAst,
    pub unit: ClauseQuantityUnitAst,
    pub distribution: ClauseDistributionAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseDestinationRelationAst {
    Into,
    From,
    To,
    Onto,
    Under,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseZonePlacementAst {
    Default,
    Top,
    Bottom,
    Shuffled,
    Tapped,
    FaceDown,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClauseDestinationAst {
    pub relation: ClauseDestinationRelationAst,
    pub zone: Zone,
    pub placement: ClauseZonePlacementAst,
    pub controller: Option<ClauseActorAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClausePredicateAst {
    Constant(bool),
    Matches {
        subject: ClauseSubjectAst,
        filter: CompilerFilterAst,
    },
    Compare {
        left: CompilerValueAst,
        operator: ValueComparisonOperator,
        right: CompilerValueAst,
    },
    ReferenceExists(SymbolReference),
    Not(Box<ClausePredicateAst>),
    All(Vec<ClausePredicateAst>),
    Any(Vec<ClausePredicateAst>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseConditionKindAst {
    If,
    Unless,
    When,
    While,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClauseConditionAst {
    pub kind: ClauseConditionKindAst,
    pub predicate: ClausePredicateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClauseDurationAst {
    Permanent,
    ThisTurn,
    UntilEndOfTurn,
    UntilEndOfCombat,
    UntilNextTurn,
    ForTurns(CompilerValueAst),
    While(ClauseConditionAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseBindingSourceAst {
    Actor,
    Subject,
    Object,
    Result,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClauseReferenceBindingAst {
    pub reference: SymbolReference,
    pub source: ClauseBindingSourceAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClauseComplementAst {
    Object(ClauseObjectAst),
    Quantity(ClauseQuantityAst),
    Destination(ClauseDestinationAst),
    Duration(ClauseDurationAst),
    Condition(ClauseConditionAst),
    Binding(ClauseReferenceBindingAst),
}

/// One semantic action assembled from reusable grammatical parts.
///
/// `provenance` deliberately does not participate in semantic identity. A
/// registry can therefore compare two clauses without erasing the source
/// evidence retained for diagnostics and faithful rendering.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerClauseAst {
    pub actor: ClauseActorAst,
    pub subject: ClauseSubjectAst,
    pub action: ClauseActionAst,
    pub object: Option<ClauseObjectAst>,
    pub quantity: Option<ClauseQuantityAst>,
    pub destination: Option<ClauseDestinationAst>,
    pub duration: Option<ClauseDurationAst>,
    pub condition: Option<ClauseConditionAst>,
    pub bindings: Vec<ClauseReferenceBindingAst>,
    pub complements: Vec<ClauseComplementAst>,
    pub library: Option<CompilerLibraryClauseAst>,
    pub object_action: Option<CompilerObjectActionClauseAst>,
    pub interaction: Option<CompilerInteractionClauseAst>,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerClauseAst {
    /// Return the semantic form used for registry deduplication and rewrite
    /// fixed points. Source evidence remains on the original clause.
    pub(crate) fn semantic_identity(&self) -> Self {
        let mut identity = self.clone();
        identity.provenance = None;
        identity.strip_nested_provenance();
        identity
    }

    pub(crate) fn semantically_equivalent(&self, other: &Self) -> bool {
        self.semantic_identity() == other.semantic_identity()
    }

    fn strip_nested_provenance(&mut self) {
        strip_actor_provenance(&mut self.actor);
        strip_subject_provenance(&mut self.subject);
        if let Some(object) = &mut self.object {
            strip_object_provenance(object);
        }
        if let Some(destination) = &mut self.destination {
            strip_destination_provenance(destination);
        }
        if let Some(condition) = &mut self.condition {
            strip_condition_provenance(condition);
        }
        if let Some(duration) = &mut self.duration {
            strip_duration_provenance(duration);
        }
        for complement in &mut self.complements {
            match complement {
                ClauseComplementAst::Object(object) => strip_object_provenance(object),
                ClauseComplementAst::Destination(destination) => {
                    strip_destination_provenance(destination)
                }
                ClauseComplementAst::Duration(duration) => strip_duration_provenance(duration),
                ClauseComplementAst::Condition(condition) => strip_condition_provenance(condition),
                ClauseComplementAst::Quantity(_) | ClauseComplementAst::Binding(_) => {}
            }
        }
        if let Some(library) = &mut self.library {
            strip_actor_provenance(&mut library.owner);
            if let Some(chooser) = &mut library.chooser {
                strip_actor_provenance(chooser);
            }
            if let Some(destination) = &mut library.destination {
                strip_destination_provenance(destination);
            }
            if let Some(remainder) = &mut library.remainder {
                strip_destination_provenance(&mut remainder.destination);
            }
        }
        if let Some(object_action) = &mut self.object_action {
            strip_object_action_provenance(object_action);
        }
        if let Some(interaction) = &mut self.interaction {
            strip_interaction_provenance(interaction);
        }
    }
}

fn strip_interaction_provenance(interaction: &mut CompilerInteractionClauseAst) {
    match interaction {
        CompilerInteractionClauseAst::Damage(damage) => {
            strip_object_operand_provenance(&mut damage.source);
            strip_object_operand_provenance(&mut damage.recipients);
        }
        CompilerInteractionClauseAst::Prevention(prevention) => {
            strip_prevention_provenance(prevention);
        }
        CompilerInteractionClauseAst::Counter(counter) => {
            strip_object_operand_provenance(&mut counter.object);
            if let Some(destination) = &mut counter.destination {
                strip_object_operand_provenance(destination);
            }
        }
        CompilerInteractionClauseAst::Combat(combat) => {
            strip_object_operand_provenance(&mut combat.primary);
            if let Some(opposing) = &mut combat.opposing {
                strip_object_operand_provenance(opposing);
            }
            if let Some(duration) = &mut combat.duration {
                strip_duration_provenance(duration);
            }
        }
        CompilerInteractionClauseAst::Characteristic(characteristic) => {
            strip_object_operand_provenance(&mut characteristic.object);
            if let Some(duration) = &mut characteristic.duration {
                strip_duration_provenance(duration);
            }
        }
    }
}

fn strip_prevention_provenance(prevention: &mut CompilerPreventionClauseAst) {
    if let Some(source) = &mut prevention.source {
        strip_object_operand_provenance(source);
    }
    if let Some(recipient) = &mut prevention.recipient {
        strip_object_operand_provenance(recipient);
    }
    if let Some(duration) = &mut prevention.duration {
        strip_duration_provenance(duration);
    }
    if let Some(redirect_to) = &mut prevention.redirect_to {
        strip_object_operand_provenance(redirect_to);
    }
}

fn strip_object_action_provenance(action: &mut CompilerObjectActionClauseAst) {
    match action {
        CompilerObjectActionClauseAst::Movement(movement) => {
            strip_object_operand_provenance(&mut movement.object);
            strip_destination_provenance(&mut movement.destination);
            strip_entry_state_provenance(&mut movement.state);
        }
        CompilerObjectActionClauseAst::Creation(creation) => {
            match &mut creation.kind {
                CompilerCreationKindAst::TokenCopy { source }
                | CompilerCreationKindAst::SpellCopy { source, .. } => {
                    strip_object_operand_provenance(source)
                }
                CompilerCreationKindAst::Token { .. } => {}
            }
            strip_actor_provenance(&mut creation.controller);
            strip_entry_state_provenance(&mut creation.state);
        }
        CompilerObjectActionClauseAst::Control(control) => {
            strip_object_operand_provenance(&mut control.object);
            strip_actor_provenance(&mut control.controller);
            if let Some(duration) = &mut control.duration {
                strip_duration_provenance(duration);
            }
            if let Some(exchange) = &mut control.exchange_with {
                strip_object_operand_provenance(exchange);
            }
        }
        CompilerObjectActionClauseAst::Attachment(attachment) => {
            strip_object_operand_provenance(&mut attachment.attachment);
            if let Some(target) = &mut attachment.target {
                strip_object_operand_provenance(target);
            }
        }
    }
}

fn strip_entry_state_provenance(state: &mut CompilerEntryStateAst) {
    if let Some(actor) = &mut state.attack_target {
        strip_actor_provenance(actor);
    }
    if let Some(attachment) = &mut state.attached_to {
        strip_object_operand_provenance(attachment);
    }
}

fn strip_object_operand_provenance(operand: &mut CompilerObjectOperandAst) {
    if let CompilerObjectOperandAst::Selection(selection) = operand {
        strip_selection_provenance(selection);
    }
}

fn strip_selection_provenance(selection: &mut CompilerSelectionAst) {
    selection.provenance = None;
}

fn strip_actor_provenance(actor: &mut ClauseActorAst) {
    if let ClauseActorAst::Selection(selection) = actor {
        strip_selection_provenance(selection);
    }
}

fn strip_subject_provenance(subject: &mut ClauseSubjectAst) {
    match subject {
        ClauseSubjectAst::Actor(actor) => strip_actor_provenance(actor),
        ClauseSubjectAst::Selection(selection) => strip_selection_provenance(selection),
        ClauseSubjectAst::Source | ClauseSubjectAst::Filter(_) | ClauseSubjectAst::Reference(_) => {
        }
    }
}

fn strip_object_provenance(object: &mut ClauseObjectAst) {
    match object {
        ClauseObjectAst::Subject(subject) => strip_subject_provenance(subject),
        ClauseObjectAst::Selection(selection) => strip_selection_provenance(selection),
        ClauseObjectAst::Cost(cost) => cost.provenance = None,
        ClauseObjectAst::Filter(_) | ClauseObjectAst::Reference(_) => {}
    }
}

fn strip_destination_provenance(destination: &mut ClauseDestinationAst) {
    if let Some(controller) = &mut destination.controller {
        strip_actor_provenance(controller);
    }
}

fn strip_duration_provenance(duration: &mut ClauseDurationAst) {
    if let ClauseDurationAst::While(condition) = duration {
        strip_condition_provenance(condition);
    }
}

fn strip_condition_provenance(condition: &mut ClauseConditionAst) {
    strip_predicate_provenance(&mut condition.predicate);
}

fn strip_predicate_provenance(predicate: &mut ClausePredicateAst) {
    match predicate {
        ClausePredicateAst::Matches { subject, .. } => strip_subject_provenance(subject),
        ClausePredicateAst::Not(predicate) => strip_predicate_provenance(predicate),
        ClausePredicateAst::All(predicates) | ClausePredicateAst::Any(predicates) => {
            for predicate in predicates {
                strip_predicate_provenance(predicate);
            }
        }
        ClausePredicateAst::Constant(_)
        | ClausePredicateAst::Compare { .. }
        | ClausePredicateAst::ReferenceExists(_) => {}
    }
}
