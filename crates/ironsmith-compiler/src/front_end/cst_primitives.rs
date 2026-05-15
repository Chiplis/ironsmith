use super::lexer::OwnedLexToken;
use super::source_model::{LineInfo, MetadataLine};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataLineCst {
    pub value: MetadataLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordLineKindCst {
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
    Multikicker,
    Replicate,
    Offspring,
    Reinforce,
    Squad,
    Transmute,
    Entwine,
    Escalate,
    Eternalize,
    Evoke,
    CastThisSpellOnly,
    Gift,
    Epic,
    Warp,
    ExertAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerIntroCst {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordLineCst {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub kind: KeywordLineKindCst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticLineCst {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementLineCst {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub parse_groups: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedLineCst {
    pub info: LineInfo,
    pub reason_code: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::TextSpan;
    use crate::front_end::{LineInfo, MetadataLine, NormalizedLine};

    fn sample_line_info() -> LineInfo {
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: "Draw a card.".to_string(),
            normalized: NormalizedLine {
                original: "Draw a card.".to_string(),
                normalized: "Draw a card.".to_string(),
                char_map: (0..12).collect(),
            },
        }
    }

    #[test]
    fn metadata_line_cst_holds_metadata_value() {
        let cst = MetadataLineCst {
            value: MetadataLine::ManaCost("{1}{U}".to_string()),
        };

        assert!(matches!(cst.value, MetadataLine::ManaCost(_)));
    }

    #[test]
    fn statement_line_cst_preserves_parse_groups() {
        let line = sample_line_info();
        let tokens = vec![
            OwnedLexToken::synthetic_word("draw"),
            OwnedLexToken::period(TextSpan::synthetic()),
        ];
        let cst = StatementLineCst {
            info: line.clone(),
            text: "Draw a card.".to_string(),
            parse_tokens: tokens.clone(),
            parse_groups: vec![tokens],
        };

        assert_eq!(cst.info, line);
        assert_eq!(cst.parse_groups.len(), 1);
    }
}
