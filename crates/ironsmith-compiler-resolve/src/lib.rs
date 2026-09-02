#![allow(clippy::too_many_arguments, clippy::type_complexity)]

//! Scoped reference resolution and compiler-AST normalization.

pub use ironsmith_compiler_semantic::model;
pub use ironsmith_compiler_semantic::model::visit::{
    assert_effect_ast_variant_coverage, for_each_nested_effects,
};
pub use ironsmith_compiler_semantic::{
    AttachmentConditionHost, AuraAttachmentFilter, ChooseSpec, ClashOpponentAst, ConditionExpr,
    ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, IfResultPredicate, KeywordAction, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst,
    PermanentLeftBattlefieldControlSurface, PlayerAst, PlayerFilter, PowerToughness,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, PtValue, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst,
    SourceCounterThresholdSurface, TagKey, TargetAst, TotalCost, ZoneReplacementDurationAst,
    ability, alternative_cast, card, cards, color, continuous, cost, costs, diagnostics, effect,
    effects, events, filter, game_state, grant, ids, mana, model_impl, object, parse_context,
    payload, resolution, static_abilities, tag, target, triggers, types, zone,
};
pub use ironsmith_core::{ObjectFilter, Value};

/// Read-only coordinate data used while attaching authored-source spans to
/// resolved semantic references.
///
/// The document phase owns the strings and mapping storage. Resolution only
/// borrows the minimal view needed to map an already-recognized span.
#[derive(Debug, Clone, Copy)]
pub struct SpanMappingContext<'a> {
    pub normalized: &'a str,
    pub original: &'a str,
    pub char_map: &'a [usize],
}

impl<'a> SpanMappingContext<'a> {
    pub const fn new(normalized: &'a str, original: &'a str, char_map: &'a [usize]) -> Self {
        Self {
            normalized,
            original,
            char_map,
        }
    }
}

pub mod util {
    pub fn source_reference_surface_for_span(
        _span: Option<crate::diagnostics::TextSpan>,
    ) -> Option<crate::target::SourceReferenceSurface> {
        None
    }

    pub fn sacrificed_object_kind_for_span(
        _span: Option<crate::diagnostics::TextSpan>,
    ) -> Option<crate::target::SacrificedObjectKind> {
        None
    }
}

pub use ironsmith_compiler_semantic::model::visit as effect_ast_traversal;

pub fn map_span_to_original(
    span: diagnostics::TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> diagnostics::TextSpan {
    fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
        text[..byte_idx.min(text.len())].chars().count()
    }
    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char > char_map.len() {
        start_orig
    } else {
        let last_orig = char_map[end_char - 1];
        last_orig
            + original_line[last_orig..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(0)
    };
    diagnostics::TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

pub mod tag_support;

pub mod compile_support {
    pub use crate::tag_support::*;
}

pub mod condition_antecedents;
pub mod effect_ast_normalization;
pub mod predicate_conditions;
pub mod reference_helpers;
pub mod reference_resolution;
pub mod trigger_players;
