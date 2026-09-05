//! Typed complements for movement, creation, copy, control, and attachment clauses.

use ironsmith_core::tag::TagKeyWalk;

use crate::color::ColorSet;
use crate::model::CompilerGrantedAbilityAst;
use crate::model::clauses::{ClauseActorAst, ClauseDestinationAst, ClauseDurationAst};
use crate::model::selections::{CompilerFilterAst, CompilerSelectionAst, CompilerValueAst};
use crate::model::symbols::SymbolReference;
use crate::model::token_definition::TokenDefinitionSpec;
use crate::object::AuraAttachmentFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CompilerObjectOperandAst {
    Source,
    Selection(CompilerSelectionAst),
    Reference(SymbolReference),
    Filter(CompilerFilterAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerControllerAst {
    Preserve,
    Owner,
    Actor,
    SourceController,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerEntryStateAst {
    pub tapped: bool,
    pub attacking: bool,
    pub attack_target: Option<ClauseActorAst>,
    pub face_down: bool,
    pub transformed: bool,
    pub cloaked: bool,
    pub attached_to: Option<CompilerObjectOperandAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerMovementClauseAst {
    pub object: CompilerObjectOperandAst,
    pub source_zones: Vec<Zone>,
    pub source_top_only: bool,
    pub destination: ClauseDestinationAst,
    pub controller: CompilerControllerAst,
    pub state: CompilerEntryStateAst,
    pub all: bool,
    pub random: bool,
    pub replacement: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompilerCopyModificationsAst {
    pub set_colors: Option<ColorSet>,
    pub set_card_types: Option<Vec<CardType>>,
    pub add_card_types: Vec<CardType>,
    pub set_subtypes: Option<Vec<Subtype>>,
    pub add_subtypes: Vec<Subtype>,
    pub remove_supertypes: Vec<Supertype>,
    pub set_base_power_toughness: Option<(CompilerValueAst, CompilerValueAst)>,
    pub half_power_toughness_round_up: bool,
    pub set_power_toughness_to_source_totals: bool,
    pub starting_loyalty: Option<u32>,
    pub has_haste: bool,
    pub loses_soulbond: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerCreationKindAst {
    Token {
        name: String,
        definition: TokenDefinitionSpec,
        dynamic_power_toughness: Option<(CompilerValueAst, CompilerValueAst)>,
        granted_abilities: Vec<CompilerGrantedAbilityAst>,
    },
    TokenCopy {
        source: CompilerObjectOperandAst,
    },
    SpellCopy {
        source: CompilerObjectOperandAst,
        may_choose_new_targets: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerDelayedDispositionAst {
    None,
    ExileEndOfCombat,
    SacrificeEndOfCombat,
    ExileNextEndStep,
    SacrificeNextEndStep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerCreationClauseAst {
    pub kind: CompilerCreationKindAst,
    pub count: CompilerValueAst,
    pub controller: ClauseActorAst,
    pub state: CompilerEntryStateAst,
    pub modifications: CompilerCopyModificationsAst,
    pub delayed_dispositions: Vec<CompilerDelayedDispositionAst>,
    pub result: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerControlClauseAst {
    pub object: CompilerObjectOperandAst,
    pub controller: ClauseActorAst,
    pub duration: Option<ClauseDurationAst>,
    pub exchange_with: Option<CompilerObjectOperandAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerAttachmentClauseAst {
    pub attachment: CompilerObjectOperandAst,
    pub target: Option<CompilerObjectOperandAst>,
    pub legality: Option<AuraAttachmentFilter>,
    pub detach: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerObjectActionClauseAst {
    Movement(CompilerMovementClauseAst),
    Creation(CompilerCreationClauseAst),
    Control(CompilerControlClauseAst),
    Attachment(CompilerAttachmentClauseAst),
}
