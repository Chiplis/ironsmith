use super::grammar::activation_costs::ActivationCostCst;
use super::lexer::OwnedLexToken;
use crate::ability::PresentationLabel;
use crate::cards::builders::{EffectAst, LineAst, ParsedLevelAbilityItemAst, PredicateAst};
use crate::ir::ChosenOptionContext;
use crate::line_info::LineInfo;
use crate::model::facts::MetadataLine;

#[derive(Debug, Clone, PartialEq)]
pub enum KeywordLinePayload {
    Ast(Box<LineAst>),
    Kicker {
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
        label: Option<String>,
    },
}

impl KeywordLinePayload {
    pub fn ast(ast: LineAst) -> Self {
        Self::Ast(Box::new(ast))
    }

    pub fn kicker(cost: ironsmith_core::TotalCost<crate::model::CompilerCost>) -> Self {
        Self::Kicker { cost, label: None }
    }

    pub fn set_kicker_label(&mut self, label: String) -> Result<(), String> {
        let Self::Kicker {
            label: current_label,
            ..
        } = self
        else {
            return Err("custom kicker label attached to a non-kicker payload".to_string());
        };
        *current_label = Some(label);
        Ok(())
    }

    pub fn to_line_ast(&self) -> LineAst {
        match self {
            Self::Ast(ast) => ast.as_ref().clone(),
            Self::Kicker { cost, label } => {
                let cost = match label {
                    Some(label) => crate::model::CompilerOptionalCost::custom(label, cost.clone()),
                    None => crate::model::CompilerOptionalCost::kicker(cost.clone()),
                };
                LineAst::OptionalCost(cost)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecognizedDocument {
    pub lines: Vec<RecognizedLine>,
}

#[derive(Debug, Clone)]
pub enum RecognizedLine {
    Metadata(RecognizedMetadataLine),
    Keyword(RecognizedKeywordLine),
    Activated(RecognizedActivatedLine),
    Triggered(RecognizedTriggeredLine),
    Static(RecognizedStaticLine),
    Statement(RecognizedStatementLine),
    Modal(RecognizedModalBlock),
    LevelHeader(RecognizedLevelHeader),
    SagaChapter(RecognizedSagaChapterLine),
    Unsupported(RecognizedUnsupportedLine),
}

#[derive(Debug, Clone)]
pub struct RecognizedMetadataLine {
    pub value: MetadataLine,
}

#[derive(Debug, Clone)]
pub struct RecognizedKeywordLine {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub kind: KeywordLineKind,
    pub payload: KeywordLinePayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordLineKind {
    AdditionalCost,
    AdditionalCostChoice,
    AlternativeCast,
    Bestow,
    Blitz,
    Bargain,
    Buyback,
    Channel,
    Craft,
    Cycling,
    Equip,
    Escape,
    Flashback,
    Harmonize,
    Kicker,
    Madness,
    Morph,
    Mutate,
    Multikicker,
    Replicate,
    Offspring,
    Reconfigure,
    Reinforce,
    Retrace,
    Squad,
    Splice,
    Transmute,
    Transfigure,
    Entwine,
    Escalate,
    Eternalize,
    Evoke,
    CastThisSpellOnly,
    Gift,
    Epic,
    Warp,
    ExertAttack,
    Exploit,
}

#[derive(Debug, Clone)]
pub struct RecognizedActivatedLine {
    pub info: LineInfo,
    pub cost: ActivationCostCst,
    pub cost_parse_tokens: Vec<OwnedLexToken>,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub presentation: Option<PresentationLabel>,
    pub chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecognizedTriggerIntro {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone)]
pub struct RecognizedTriggeredLine {
    pub info: LineInfo,
    pub full_text: String,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub trigger_parse_tokens: Vec<OwnedLexToken>,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub intervening_if: Option<PredicateAst>,
    pub max_triggers_per_turn: Option<u32>,
    pub chosen_option: Option<ChosenOptionContext>,
    pub presentation: Option<PresentationLabel>,
}

#[derive(Debug, Clone)]
pub struct RecognizedStaticLine {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub chosen_option: Option<ChosenOptionContext>,
    pub parsed: Option<Box<LineAst>>,
}

#[derive(Debug, Clone)]
pub struct RecognizedStatementLine {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub parse_groups: Vec<Vec<OwnedLexToken>>,
    /// Effects produced by a grammar-specific statement recognizer.
    ///
    /// When present, semantic assembly consumes this typed result directly
    /// instead of sending the same token group through the broad effect
    /// dispatcher a second time.
    pub parsed_effects: Option<Vec<EffectAst>>,
}

#[derive(Debug, Clone)]
pub struct RecognizedModalBlock {
    pub header: LineInfo,
    pub header_tokens: Vec<OwnedLexToken>,
    pub modes: Vec<RecognizedModalMode>,
}

#[derive(Debug, Clone)]
pub struct RecognizedModalMode {
    pub info: LineInfo,
    pub text: String,
    pub point_cost: Option<u32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct RecognizedLevelHeader {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<RecognizedLevelItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelItemKind {
    KeywordActions,
    StaticAbilities,
    ActivatedAbility,
}

#[derive(Debug, Clone)]
pub struct RecognizedLevelItem {
    pub info: LineInfo,
    pub text: String,
    pub kind: LevelItemKind,
    pub parsed: ParsedLevelAbilityItemAst,
}

#[derive(Debug, Clone)]
pub struct RecognizedSagaChapterLine {
    pub info: LineInfo,
    pub chapters: Vec<u32>,
    pub presentation_label: Option<String>,
    pub text: String,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct RecognizedUnsupportedLine {
    pub info: LineInfo,
    pub reason_code: &'static str,
}

#[cfg(test)]
mod layout_tests {
    fn assert_layout_bound<T>(name: &str, maximum: usize) {
        let actual = std::mem::size_of::<T>();
        assert!(
            actual <= maximum,
            "{name} grew to {actual} bytes; phase-boundary records must remain at most {maximum} bytes"
        );
    }

    #[test]
    fn frontend_phase_records_stay_boxed_at_boundaries() {
        assert_layout_bound::<super::RecognizedLine>("RecognizedLine", 4_096);
        assert_layout_bound::<super::LineInfo>("LineInfo", 1_024);
        assert_layout_bound::<crate::model::facts::LineSemanticFacts>("LineSemanticFacts", 1_024);
        assert_layout_bound::<crate::ir::RewriteSemanticItem>("RewriteSemanticItem", 1_024);
        assert_layout_bound::<crate::model::compiler_semantic::ParsedLineAst>(
            "ParsedLineAst",
            1_024,
        );
    }
}
