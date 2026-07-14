pub(crate) use crate::cards::builders::{
    EffectAst, OwnedLexToken, PredicateAst, SubjectAst, TriggerSpec,
};
pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use crate::model::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExchangeValueKindAst,
    ExtraTurnAnchorAst, FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, ParsedAbility,
    ParsedRestrictions, PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    RetargetModeAst, ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
pub use crate::payload::{IfResultPredicate, KeywordAction};
pub use ironsmith_core::TagKey;

pub const IT_TAG: &str = "__it__";
/// Parse-time marker for a passive "was sacrificed this way" reference.
///
/// Unlike a bare `IT_TAG`, this remains tied to the sacrifice event even when
/// an intervening instruction establishes the source as the newest ordinary
/// pronoun antecedent.
pub const THIS_WAY_SACRIFICED_TAG: &str = "__this_way_sacrificed__";
pub const CHOSEN_OBJECTS_TAG: &str = "__chosen_objects__";
pub const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";
