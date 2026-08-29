pub(super) use super::grammar::keyword_dispatch::{
    KeywordDispatchHint, parse_keyword_dispatch_hint_tokens as parse_keyword_dispatch_hint,
};
use super::keyword_registry as registry;
use super::lexer::OwnedLexToken;
use crate::recognition::RuleId;

pub(super) type KeywordParseFn = fn(
    &super::preprocess::PreprocessedLine,
    &[OwnedLexToken],
    &[OwnedLexToken],
) -> Result<
    Option<super::recognized_document::KeywordLinePayload>,
    crate::cards::builders::CardTextError,
>;

#[derive(Clone, Copy)]
pub(super) struct KeywordLineRule {
    pub(super) id: RuleId,
    pub(super) cst_kind: super::recognized_document::KeywordLineKind,
    pub(super) hints: &'static [KeywordDispatchHint],
    pub(super) parse: KeywordParseFn,
}

mod additional_costs {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::AdditionalCostChoice,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            id: RuleId::new("parse_additional_cost_choice"),
            parse: registry::parse_additional_cost_choice,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::AdditionalCost,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            id: RuleId::new("parse_additional_cost"),
            parse: registry::parse_additional_cost,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::CastThisSpellOnly,
            hints: &[KeywordDispatchHint::CastThisSpellOnly],
            id: RuleId::new("parse_cast_this_spell_only"),
            parse: registry::parse_cast_this_spell_only,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Gift,
            hints: &[KeywordDispatchHint::Gift],
            id: RuleId::new("parse_gift"),
            parse: registry::parse_gift,
        },
    ];
}

mod activated_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Channel,
            hints: &[KeywordDispatchHint::Channel],
            id: RuleId::new("parse_channel"),
            parse: registry::parse_channel,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Cycling,
            hints: &[KeywordDispatchHint::Cycling],
            id: RuleId::new("parse_cycling"),
            parse: registry::parse_cycling,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Craft,
            hints: &[KeywordDispatchHint::Craft],
            id: RuleId::new("parse_craft"),
            parse: registry::parse_craft,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Reinforce,
            hints: &[KeywordDispatchHint::Reinforce],
            id: RuleId::new("parse_reinforce"),
            parse: registry::parse_reinforce,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Equip,
            hints: &[KeywordDispatchHint::Equip],
            id: RuleId::new("parse_equip"),
            parse: registry::parse_equip,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Reconfigure,
            hints: &[KeywordDispatchHint::Reconfigure],
            id: RuleId::new("parse_reconfigure"),
            parse: registry::parse_reconfigure,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Eternalize,
            hints: &[KeywordDispatchHint::Eternalize],
            id: RuleId::new("parse_eternalize"),
            parse: registry::parse_eternalize,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Morph,
            hints: &[KeywordDispatchHint::MorphFamily],
            id: RuleId::new("parse_morph"),
            parse: registry::parse_morph,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Mutate,
            hints: &[KeywordDispatchHint::Mutate],
            id: RuleId::new("parse_mutate"),
            parse: registry::parse_mutate,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Transmute,
            hints: &[KeywordDispatchHint::Transmute],
            id: RuleId::new("parse_transmute"),
            parse: registry::parse_transmute,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Transfigure,
            hints: &[KeywordDispatchHint::Transfigure],
            id: RuleId::new("parse_transfigure"),
            parse: registry::parse_transfigure,
        },
    ];
}

mod spell_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::AlternativeCast,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            id: RuleId::new("parse_alternative_cast"),
            parse: registry::parse_alternative_cast,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Bestow,
            hints: &[KeywordDispatchHint::Bestow],
            id: RuleId::new("parse_bestow"),
            parse: registry::parse_bestow,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Blitz,
            hints: &[KeywordDispatchHint::Blitz],
            id: RuleId::new("parse_blitz"),
            parse: registry::parse_blitz,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Bargain,
            hints: &[KeywordDispatchHint::Bargain],
            id: RuleId::new("parse_bargain"),
            parse: registry::parse_bargain,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Buyback,
            hints: &[KeywordDispatchHint::Buyback],
            id: RuleId::new("parse_buyback"),
            parse: registry::parse_buyback,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Kicker,
            hints: &[KeywordDispatchHint::Kicker],
            id: RuleId::new("parse_kicker"),
            parse: registry::parse_kicker,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Flashback,
            hints: &[KeywordDispatchHint::Flashback],
            id: RuleId::new("parse_flashback"),
            parse: registry::parse_flashback,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Harmonize,
            hints: &[KeywordDispatchHint::Harmonize],
            id: RuleId::new("parse_harmonize"),
            parse: registry::parse_harmonize,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Retrace,
            hints: &[KeywordDispatchHint::Retrace],
            id: RuleId::new("parse_retrace"),
            parse: registry::parse_retrace,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Multikicker,
            hints: &[KeywordDispatchHint::Multikicker],
            id: RuleId::new("parse_multikicker"),
            parse: registry::parse_multikicker,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Replicate,
            hints: &[KeywordDispatchHint::Replicate],
            id: RuleId::new("parse_replicate"),
            parse: registry::parse_replicate,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Entwine,
            hints: &[KeywordDispatchHint::Entwine],
            id: RuleId::new("parse_entwine"),
            parse: registry::parse_entwine,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Escalate,
            hints: &[KeywordDispatchHint::Escalate],
            id: RuleId::new("parse_escalate"),
            parse: registry::parse_escalate,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Evoke,
            hints: &[KeywordDispatchHint::Evoke],
            id: RuleId::new("parse_evoke"),
            parse: registry::parse_evoke,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Epic,
            hints: &[KeywordDispatchHint::Epic],
            id: RuleId::new("parse_epic"),
            parse: registry::parse_epic,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Offspring,
            hints: &[KeywordDispatchHint::Offspring],
            id: RuleId::new("parse_offspring"),
            parse: registry::parse_offspring,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Madness,
            hints: &[KeywordDispatchHint::Madness],
            id: RuleId::new("parse_madness"),
            parse: registry::parse_madness,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Escape,
            hints: &[KeywordDispatchHint::Escape],
            id: RuleId::new("parse_escape"),
            parse: registry::parse_escape,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Squad,
            hints: &[KeywordDispatchHint::Squad],
            id: RuleId::new("parse_squad"),
            parse: registry::parse_squad,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Splice,
            hints: &[KeywordDispatchHint::Splice],
            id: RuleId::new("parse_splice"),
            parse: registry::parse_splice,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Warp,
            hints: &[KeywordDispatchHint::Warp],
            id: RuleId::new("parse_warp"),
            parse: registry::parse_warp,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::ExertAttack,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            id: RuleId::new("parse_exert_attack"),
            parse: registry::parse_exert_attack,
        },
        KeywordLineRule {
            cst_kind: super::super::recognized_document::KeywordLineKind::Exploit,
            hints: &[KeywordDispatchHint::Exploit],
            id: RuleId::new("parse_exploit"),
            parse: registry::parse_exploit,
        },
    ];
}

pub(super) fn keyword_line_rules() -> Vec<KeywordLineRule> {
    let mut rules = Vec::new();
    rules.extend_from_slice(additional_costs::RULES);
    rules.extend_from_slice(activated_keywords::RULES);
    rules.extend_from_slice(spell_keywords::RULES);
    rules
}
