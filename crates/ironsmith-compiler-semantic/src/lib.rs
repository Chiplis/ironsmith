// Canonical semantic nodes intentionally carry complete value/filter subtrees. Boxing each
// variant solely for lint-size uniformity would make the shared AST API allocation-driven.
#![allow(
    clippy::large_enum_variant,
    clippy::too_many_arguments,
    clippy::type_complexity
)]
#![allow(ambiguous_glob_reexports)]

//! Recursive compiled-value graph shared by compiler grammar and lowering.

pub use ironsmith_core::{
    AttachmentConditionHost, Condition as ConditionExpr, PermanentLeftBattlefieldControlSurface,
    SourceCounterThresholdSurface,
};

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}
pub use cost::TotalCost;
pub use ironsmith_compiler_ast::parse_context;
pub use ironsmith_compiler_ast::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RetargetModeAst, ReturnControllerAst, SearchLibrarySlotAst,
    SharedTypeConstraintAst, TargetAst, ZoneReplacementDurationAst,
};
pub use payload::{IfResultPredicate, KeywordAction};
pub use tag::TagKey;
pub use target::{ChooseSpec, PlayerFilter};

pub mod ability;
pub mod alternative_cast;
pub mod card;
pub mod card_document;
pub mod color;
pub mod condition_antecedent;
pub mod continuous;
pub mod cost;
pub mod costs;
pub mod effect;
pub mod effects;
pub mod events;
pub mod filter;
pub mod game_state;
pub mod grant;
pub mod ids;
pub mod keyword_abilities;
pub mod mana;
pub mod object;
pub mod payload;
pub mod resolution;
pub mod static_abilities;
pub mod tag;
pub mod target;
pub mod trigger_references;
pub mod triggers;
pub mod types;
pub mod zone;

pub mod cards {
    pub use crate::diagnostics::{ParseAnnotations, TextSpan};
    pub type CardDefinition = ironsmith_core::CardDefinition<
        crate::ability::Ability,
        crate::effect::Effect,
        crate::costs::Cost,
        crate::alternative_cast::AlternativeCastingMethod,
        crate::cost::OptionalCost,
    >;

    pub mod builders {
        pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
        pub use crate::model::ast::*;
        pub use crate::model::facts::*;
        pub use crate::model::*;
        pub use crate::payload::KeywordAction;
        pub use crate::static_abilities::StaticAbility;
        pub use crate::tag::TagKey;

        #[derive(Debug, Clone, PartialEq)]
        pub enum GrantedAbilityAst {
            KeywordAction(Box<KeywordAction>),
            StaticAbility(Box<StaticAbilityAst>),
            ThisAbility,
            MustAttack,
            MustBlock,
            CanAttackAsThoughNoDefender,
            CanBlockAdditionalCreatureEachCombat {
                additional: usize,
            },
            ParsedObjectAbility {
                ability: Box<crate::model::compiler_semantic::ParsedAbility>,
                display: String,
            },
        }

        impl From<KeywordAction> for GrantedAbilityAst {
            fn from(action: KeywordAction) -> Self {
                Self::KeywordAction(Box::new(action))
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum InsteadSemantics {
            SelfReplacement,
            FutureReplacement,
            NonReplacement,
        }
    }
}

pub mod model_impl;
pub use model_impl as model;

pub use card::PowerToughness;
pub use card::PtValue;
pub use object::AuraAttachmentFilter;
