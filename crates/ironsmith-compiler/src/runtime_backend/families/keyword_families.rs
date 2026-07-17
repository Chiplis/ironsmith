use super::grammar::keyword_dispatch as keyword_dispatch_grammar;
pub(super) use super::grammar::keyword_dispatch::KeywordDispatchHint;
use super::keyword_registry as registry;
use super::lexer::OwnedLexToken;

pub(super) type KeywordParseFn =
    fn(
        &super::preprocess::PreprocessedLine,
        &[OwnedLexToken],
        &[OwnedLexToken],
    )
        -> Result<Option<super::cst::KeywordLinePayloadCst>, crate::cards::builders::CardTextError>;

#[derive(Clone, Copy)]
pub(super) struct KeywordLineRule {
    pub(super) cst_kind: super::cst::KeywordLineKindCst,
    pub(super) hints: &'static [KeywordDispatchHint],
    pub(super) parse: KeywordParseFn,
}

mod additional_costs {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AdditionalCostChoice,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            parse: registry::parse_additional_cost_choice,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AdditionalCost,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            parse: registry::parse_additional_cost,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::CastThisSpellOnly,
            hints: &[KeywordDispatchHint::CastThisSpellOnly],
            parse: registry::parse_cast_this_spell_only,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Gift,
            hints: &[KeywordDispatchHint::Gift],
            parse: registry::parse_gift,
        },
    ];
}

mod activated_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Channel,
            hints: &[KeywordDispatchHint::Channel],
            parse: registry::parse_channel,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Cycling,
            hints: &[KeywordDispatchHint::Cycling],
            parse: registry::parse_cycling,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Craft,
            hints: &[KeywordDispatchHint::Craft],
            parse: registry::parse_craft,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Reinforce,
            hints: &[KeywordDispatchHint::Reinforce],
            parse: registry::parse_reinforce,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Equip,
            hints: &[KeywordDispatchHint::Equip],
            parse: registry::parse_equip,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Reconfigure,
            hints: &[KeywordDispatchHint::Reconfigure],
            parse: registry::parse_reconfigure,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Eternalize,
            hints: &[KeywordDispatchHint::Eternalize],
            parse: registry::parse_eternalize,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Morph,
            hints: &[KeywordDispatchHint::MorphFamily],
            parse: registry::parse_morph,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Mutate,
            hints: &[KeywordDispatchHint::Mutate],
            parse: registry::parse_mutate,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Transmute,
            hints: &[KeywordDispatchHint::Transmute],
            parse: registry::parse_transmute,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Transfigure,
            hints: &[KeywordDispatchHint::Transfigure],
            parse: registry::parse_transfigure,
        },
    ];
}

mod spell_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AlternativeCast,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            parse: registry::parse_alternative_cast,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Bestow,
            hints: &[KeywordDispatchHint::Bestow],
            parse: registry::parse_bestow,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Blitz,
            hints: &[KeywordDispatchHint::Blitz],
            parse: registry::parse_blitz,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Bargain,
            hints: &[KeywordDispatchHint::Bargain],
            parse: registry::parse_bargain,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Buyback,
            hints: &[KeywordDispatchHint::Buyback],
            parse: registry::parse_buyback,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Kicker,
            hints: &[KeywordDispatchHint::Kicker],
            parse: registry::parse_kicker,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Flashback,
            hints: &[KeywordDispatchHint::Flashback],
            parse: registry::parse_flashback,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Harmonize,
            hints: &[KeywordDispatchHint::Harmonize],
            parse: registry::parse_harmonize,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Retrace,
            hints: &[KeywordDispatchHint::Retrace],
            parse: registry::parse_retrace,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Multikicker,
            hints: &[KeywordDispatchHint::Multikicker],
            parse: registry::parse_multikicker,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Replicate,
            hints: &[KeywordDispatchHint::Replicate],
            parse: registry::parse_replicate,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Entwine,
            hints: &[KeywordDispatchHint::Entwine],
            parse: registry::parse_entwine,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Escalate,
            hints: &[KeywordDispatchHint::Escalate],
            parse: registry::parse_escalate,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Evoke,
            hints: &[KeywordDispatchHint::Evoke],
            parse: registry::parse_evoke,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Epic,
            hints: &[KeywordDispatchHint::Epic],
            parse: registry::parse_epic,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Offspring,
            hints: &[KeywordDispatchHint::Offspring],
            parse: registry::parse_offspring,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Madness,
            hints: &[KeywordDispatchHint::Madness],
            parse: registry::parse_madness,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Escape,
            hints: &[KeywordDispatchHint::Escape],
            parse: registry::parse_escape,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Squad,
            hints: &[KeywordDispatchHint::Squad],
            parse: registry::parse_squad,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Splice,
            hints: &[KeywordDispatchHint::Splice],
            parse: registry::parse_splice,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Warp,
            hints: &[KeywordDispatchHint::Warp],
            parse: registry::parse_warp,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::ExertAttack,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            parse: registry::parse_exert_attack,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Exploit,
            hints: &[KeywordDispatchHint::Exploit],
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

pub(super) fn parse_keyword_dispatch_hint(tokens: &[OwnedLexToken]) -> Option<KeywordDispatchHint> {
    keyword_dispatch_grammar::parse_keyword_dispatch_hint_tokens(tokens)
}
