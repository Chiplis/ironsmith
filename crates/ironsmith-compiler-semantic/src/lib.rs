#![expect(clippy::type_complexity, clippy::too_many_arguments)]
#![allow(ambiguous_glob_reexports)]

//! Recursive compiled-value graph shared by compiler grammar and lowering.

pub use ironsmith_core::{
    AttachmentConditionHost, Condition as ConditionExpr, PermanentLeftBattlefieldControlSurface,
    SourceCounterThresholdSurface,
};

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}
pub use ironsmith_compiler_ast::parse_context;
pub mod front_end {
    pub use ironsmith_compiler_source::*;
    pub use ironsmith_compiler_syntax::*;
}
pub use cost::TotalCost;
pub use ironsmith_compiler_ast::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RetargetModeAst, ReturnControllerAst, SearchLibrarySlotAst,
    SharedTypeConstraintAst, TargetAst, ZoneReplacementDurationAst,
};
pub use ironsmith_compiler_syntax::lexer;
pub use payload::{IfResultPredicate, KeywordAction};
pub use tag::TagKey;
pub use target::{ChooseSpec, PlayerFilter};

#[path = "../../ironsmith-compiler/src/ability.rs"]
pub mod ability;
#[path = "../../ironsmith-compiler/src/alternative_cast.rs"]
pub mod alternative_cast;
#[path = "../../ironsmith-compiler/src/card.rs"]
pub mod card;
#[path = "../../ironsmith-compiler/src/color.rs"]
pub mod color;
#[path = "../../ironsmith-compiler/src/continuous.rs"]
pub mod continuous;
#[path = "../../ironsmith-compiler/src/cost.rs"]
pub mod cost;
#[path = "../../ironsmith-compiler/src/costs/mod.rs"]
pub mod costs;
#[path = "../../ironsmith-compiler/src/effect.rs"]
pub mod effect;
#[path = "../../ironsmith-compiler/src/effects/mod.rs"]
pub mod effects;
#[path = "../../ironsmith-compiler/src/events.rs"]
pub mod events;
#[path = "../../ironsmith-compiler/src/filter.rs"]
pub mod filter;
#[path = "../../ironsmith-compiler/src/game_state.rs"]
pub mod game_state;
#[path = "../../ironsmith-compiler/src/grant.rs"]
pub mod grant;
#[path = "../../ironsmith-compiler/src/ids.rs"]
pub mod ids;
#[path = "../../ironsmith-compiler/src/mana.rs"]
pub mod mana;
#[path = "../../ironsmith-compiler/src/object.rs"]
pub mod object;
#[path = "../../ironsmith-compiler/src/payload.rs"]
pub mod payload;
#[path = "../../ironsmith-compiler/src/resolution.rs"]
pub mod resolution;
#[path = "../../ironsmith-compiler/src/static_abilities.rs"]
pub mod static_abilities;
#[path = "../../ironsmith-compiler/src/tag.rs"]
pub mod tag;
#[path = "../../ironsmith-compiler/src/target.rs"]
pub mod target;
#[path = "../../ironsmith-compiler/src/triggers.rs"]
pub mod triggers;
#[path = "../../ironsmith-compiler/src/types.rs"]
pub mod types;
#[path = "../../ironsmith-compiler/src/zone.rs"]
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
            KeywordAction(KeywordAction),
            StaticAbility(StaticAbility),
            ThisAbility,
            MustAttack,
            MustBlock,
            CanAttackAsThoughNoDefender,
            CanBlockAdditionalCreatureEachCombat {
                additional: usize,
            },
            ParsedObjectAbility {
                ability: crate::model::compiler_semantic::ParsedAbility,
                display: String,
            },
        }

        impl From<KeywordAction> for GrantedAbilityAst {
            fn from(action: KeywordAction) -> Self {
                Self::KeywordAction(action)
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum InsteadSemantics {
            SelfReplacement,
            FutureReplacement,
            NonReplacement,
        }

        pub const IT_TAG: &str = "__it__";
        pub const ADDITIONAL_COST_OBJECT_TAG: &str = "__additional_cost_object__";
        pub const THIS_WAY_SACRIFICED_TAG: &str = "__this_way_sacrificed__";
        pub const CHOSEN_OBJECTS_TAG: &str = ironsmith_core::CHOSEN_OBJECTS_TAG;
        pub const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";
    }
}

pub mod model_impl;
pub use model_impl as model;

pub use card::PowerToughness;
pub use card::PtValue;
pub use object::AuraAttachmentFilter;
