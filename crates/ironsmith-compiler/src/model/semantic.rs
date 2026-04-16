use crate::front_end::LineInfo;
use crate::model::{ParsedRestrictions, ReferenceImports};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiftTimingAst {
    SpellResolution,
    PermanentEtb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineAst<
    KeywordAction,
    StaticAbility,
    ParsedAbility,
    Effect,
    TriggerSpec,
    OptionalCost,
    AlternativeCastingMethod,
> {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbility),
    StaticAbilities(Vec<StaticAbility>),
    Ability(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<Effect>,
        max_triggers_per_turn: Option<u32>,
    },
    Statement {
        effects: Vec<Effect>,
    },
    AdditionalCost {
        effects: Vec<Effect>,
    },
    OptionalCost(OptionalCost),
    GiftKeyword {
        cost: OptionalCost,
        effects: Vec<Effect>,
        followup_text: String,
        timing: GiftTimingAst,
    },
    OptionalCostWithCastTrigger {
        cost: OptionalCost,
        effects: Vec<Effect>,
        followup_text: String,
    },
    AdditionalCostChoice {
        options: Vec<AdditionalCostChoiceOptionAst<Effect>>,
    },
    AlternativeCastingMethod(AlternativeCastingMethod),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditionalCostChoiceOptionAst<Effect = crate::effect::Effect> {
    pub description: String,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAbility<Ability, Effect, PlayerFilter, TriggerSpec> {
    pub ability: Ability,
    pub effects_ast: Option<Vec<Effect>>,
    pub reference_imports: ReferenceImports<PlayerFilter>,
    pub trigger_spec: Option<TriggerSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCardItem<Line, Modal, Level> {
    pub item: ParsedCardItemKind<Line, Modal, Level>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCardItemKind<Line, Modal, Level> {
    Line(Line),
    Modal(Modal),
    LevelAbility(Level),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLineAst<Chunk> {
    pub info: LineInfo,
    pub chunks: Vec<Chunk>,
    pub restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModalAst<TriggerSpec, Effect, Value, Predicate, TotalCost, Zone, Condition> {
    pub header:
        ParsedModalHeader<TriggerSpec, Effect, Value, Predicate, TotalCost, Zone, Condition>,
    pub modes: Vec<ParsedModalModeAst<Effect>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModalHeader<TriggerSpec, Effect, Value, Predicate, TotalCost, Zone, Condition> {
    pub min: Value,
    pub max: Option<Value>,
    pub same_mode_more_than_once: bool,
    pub mode_must_be_unchosen: bool,
    pub mode_must_be_unchosen_this_turn: bool,
    pub commander_allows_both: bool,
    pub trigger: Option<TriggerSpec>,
    pub activated: Option<ParsedModalActivatedHeader<TotalCost, Zone, Condition>>,
    pub x_replacement: Option<Value>,
    pub prefix_effects_ast: Vec<Effect>,
    pub modal_gate: Option<ParsedModalGate<Predicate>>,
    pub line_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModalActivatedHeader<TotalCost, Zone, Condition> {
    pub mana_cost: TotalCost,
    pub functional_zones: Vec<Zone>,
    pub timing: String,
    pub additional_restrictions: Vec<String>,
    pub activation_restrictions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModalModeAst<Effect> {
    pub info: LineInfo,
    pub description: String,
    pub effects_ast: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModalGate<Predicate> {
    pub predicate: Predicate,
    pub remove_mode_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLevelAbilityAst<Item> {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLevelAbilityItemAst<StaticAbility, KeywordAction> {
    StaticAbilities(Vec<StaticAbility>),
    KeywordActions(Vec<KeywordAction>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_line_ast_keeps_chunks_and_restrictions() {
        let line = ParsedLineAst {
            info: LineInfo {
                line_index: 2,
                display_line_index: 2,
                raw_line: "Draw a card.".to_string(),
                normalized: crate::front_end::NormalizedLine {
                    original: "Draw a card.".to_string(),
                    normalized: "Draw a card.".to_string(),
                    char_map: (0..12).collect(),
                },
            },
            chunks: vec!["draw".to_string()],
            restrictions: ParsedRestrictions::default(),
        };

        assert_eq!(line.chunks, vec!["draw"]);
        assert!(line.restrictions.is_empty());
    }

    #[test]
    fn line_ast_can_hold_triggered_chunk() {
        let chunk = LineAst::<String, String, String, String, String, String, String>::Triggered {
            trigger: "when this enters".to_string(),
            effects: vec!["draw".to_string()],
            max_triggers_per_turn: Some(1),
        };

        assert!(matches!(
            chunk,
            LineAst::Triggered {
                max_triggers_per_turn: Some(1),
                ..
            }
        ));
    }
}
