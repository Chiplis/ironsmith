//! The AST vocabulary recognition writes into.
//!
//! Recognition names these types through one module so the grammar reads as one
//! vocabulary rather than a scatter of import paths. Every name here is either
//! owned by the semantic crate or by the grammar's own recognition helpers —
//! the card definition builder is not among them, because building the
//! definition is lowering's job.

pub use crate::ability::{ActivationTiming, PresentationKeyword, PresentationLabel};
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
use crate::color::ColorSet;
pub use crate::cost::OptionalCost;
use crate::cost::TotalCost;
pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use crate::effect::EffectPredicate;
pub use crate::effect::{ChoiceCount, EventValueSpec, Value};
pub use crate::effect_sentences::{CarryContext, TokenCopyFollowup, Verb};
#[cfg(test)]
pub use crate::effect_sentences::{
    find_verb, parse_effect_sentence_lexed, parse_shared_color_target_fanout_sentence,
};
pub use crate::line_info::LineInfo;
use crate::mana::ManaCost;
pub use crate::model::compiler_semantic::{
    ConditionalModeSelection, GiftTimingAst, LineAst, ParsedAbility, ParsedCardItem,
    ParsedConditionalModeChange, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst,
    ParsedLevelActivatedAbilityAst, ParsedLineAst, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
};
pub use crate::model::facts::{EffectLoweringContext, IdGenContext, LoweringFrame, MetadataLine};
pub use crate::model::reference::RefState;
pub use crate::model::reference_state::{ReferenceEnv, ReferenceImports};
pub use crate::model::{
    AdditionalCostChoiceOptionAst, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RedirectNextTimeDamageDestinationAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use crate::object::AuraAttachmentFilter;
pub use crate::payload::{IfResultPredicate, KeywordAction};
pub use crate::permission_helpers::{PermissionClauseSpec, PermissionLifetime};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
pub use crate::tag::TagKey;
pub use crate::target::{ObjectFilter, PlayerFilter};
pub use crate::types::CardType;
use crate::types::{Subtype, Supertype};
pub use crate::util::SubjectAst;
pub use ironsmith_compiler_semantic::cards::CardDefinition;
pub use ironsmith_compiler_source::NormalizedLine;
pub use ironsmith_core::CardId;

#[cfg(test)]
pub mod document_parser {
    pub use crate::recognized_document::KeywordLineKind;
}

pub use ironsmith_compiler_semantic::cards::builders::GrantedAbilityAst;

pub use crate::lexer::OwnedLexToken;

pub use crate::model::ast::{
    ChooseOneModeAst, EffectAst, PredicateAst, StaticAbilityAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TriggerFrequencyPredicateAst,
    TriggerSpec, TurnHistoryPredicateAst,
};

pub use ironsmith_compiler_semantic::cards::builders::InsteadSemantics;
