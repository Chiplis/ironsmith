use winnow::Parser;

use super::grammar::primitives::{self as grammar, TokenWordView};
use super::keyword_registry as registry;
use super::lexer::OwnedLexToken;
use super::token_primitives::str_strip_suffix;

pub(super) type KeywordRuleFn =
    fn(&super::preprocess::PreprocessedLine, &[OwnedLexToken]) -> Result<bool, crate::cards::builders::CardTextError>;
pub(super) type KeywordLowerFn =
    fn(&super::ir::RewriteKeywordLine, &[OwnedLexToken]) -> Result<crate::cards::builders::LineAst, crate::cards::builders::CardTextError>;

#[derive(Clone, Copy)]
pub(super) struct KeywordLineRule {
    pub(super) cst_kind: super::cst::KeywordLineKindCst,
    pub(super) hints: &'static [KeywordDispatchHint],
    pub(super) matches: KeywordRuleFn,
    pub(super) lower: KeywordLowerFn,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum KeywordDispatchHint {
    AdditionalCostFamily,
    AlternativeOrExertFamily,
    Bestow,
    Bargain,
    Buyback,
    Channel,
    Cycling,
    Reinforce,
    Equip,
    Kicker,
    Flashback,
    Harmonize,
    Multikicker,
    Entwine,
    Offspring,
    Madness,
    Escape,
    MorphFamily,
    Squad,
    Transmute,
    CastThisSpellOnly,
    Gift,
    Warp,
}

mod additional_costs {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AdditionalCostChoice,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            matches: registry::matches_additional_cost_choice,
            lower: registry::lower_additional_cost_choice,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AdditionalCost,
            hints: &[KeywordDispatchHint::AdditionalCostFamily],
            matches: registry::matches_additional_cost,
            lower: registry::lower_additional_cost,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::CastThisSpellOnly,
            hints: &[KeywordDispatchHint::CastThisSpellOnly],
            matches: registry::matches_cast_this_spell_only,
            lower: registry::lower_cast_this_spell_only,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Gift,
            hints: &[KeywordDispatchHint::Gift],
            matches: registry::matches_gift,
            lower: registry::lower_gift,
        },
    ];
}

mod activated_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Channel,
            hints: &[KeywordDispatchHint::Channel],
            matches: registry::matches_channel,
            lower: registry::lower_channel,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Cycling,
            hints: &[KeywordDispatchHint::Cycling],
            matches: registry::matches_cycling,
            lower: registry::lower_cycling,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Reinforce,
            hints: &[KeywordDispatchHint::Reinforce],
            matches: registry::matches_reinforce,
            lower: registry::lower_reinforce,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Equip,
            hints: &[KeywordDispatchHint::Equip],
            matches: registry::matches_equip,
            lower: registry::lower_equip,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Morph,
            hints: &[KeywordDispatchHint::MorphFamily],
            matches: registry::matches_morph,
            lower: registry::lower_morph,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Transmute,
            hints: &[KeywordDispatchHint::Transmute],
            matches: registry::matches_transmute,
            lower: registry::lower_transmute,
        },
    ];
}

mod spell_keywords {
    use super::*;

    pub(super) const RULES: &[KeywordLineRule] = &[
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::AlternativeCast,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            matches: registry::matches_alternative_cast,
            lower: registry::lower_alternative_cast,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Bestow,
            hints: &[KeywordDispatchHint::Bestow],
            matches: registry::matches_bestow,
            lower: registry::lower_bestow,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Bargain,
            hints: &[KeywordDispatchHint::Bargain],
            matches: registry::matches_bargain,
            lower: registry::lower_bargain,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Buyback,
            hints: &[KeywordDispatchHint::Buyback],
            matches: registry::matches_buyback,
            lower: registry::lower_buyback,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Kicker,
            hints: &[KeywordDispatchHint::Kicker],
            matches: registry::matches_kicker,
            lower: registry::lower_kicker,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Flashback,
            hints: &[KeywordDispatchHint::Flashback],
            matches: registry::matches_flashback,
            lower: registry::lower_flashback,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Harmonize,
            hints: &[KeywordDispatchHint::Harmonize],
            matches: registry::matches_harmonize,
            lower: registry::lower_harmonize,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Multikicker,
            hints: &[KeywordDispatchHint::Multikicker],
            matches: registry::matches_multikicker,
            lower: registry::lower_multikicker,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Entwine,
            hints: &[KeywordDispatchHint::Entwine],
            matches: registry::matches_entwine,
            lower: registry::lower_entwine,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Offspring,
            hints: &[KeywordDispatchHint::Offspring],
            matches: registry::matches_offspring,
            lower: registry::lower_offspring,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Madness,
            hints: &[KeywordDispatchHint::Madness],
            matches: registry::matches_madness,
            lower: registry::lower_madness,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Escape,
            hints: &[KeywordDispatchHint::Escape],
            matches: registry::matches_escape,
            lower: registry::lower_escape,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Squad,
            hints: &[KeywordDispatchHint::Squad],
            matches: registry::matches_squad,
            lower: registry::lower_squad,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::Warp,
            hints: &[KeywordDispatchHint::Warp],
            matches: registry::matches_warp,
            lower: registry::lower_warp,
        },
        KeywordLineRule {
            cst_kind: super::super::cst::KeywordLineKindCst::ExertAttack,
            hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
            matches: registry::matches_exert_attack,
            lower: registry::lower_exert_attack,
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
    let hinted = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            winnow::combinator::alt((
                grammar::phrase(&[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                ])
                .value(KeywordDispatchHint::AdditionalCostFamily),
                grammar::kw("you").value(KeywordDispatchHint::AlternativeOrExertFamily),
                grammar::kw("if").value(KeywordDispatchHint::AlternativeOrExertFamily),
                grammar::kw("bestow").value(KeywordDispatchHint::Bestow),
                grammar::kw("bargain").value(KeywordDispatchHint::Bargain),
                grammar::kw("buyback").value(KeywordDispatchHint::Buyback),
                grammar::kw("channel").value(KeywordDispatchHint::Channel),
                grammar::kw("cycling").value(KeywordDispatchHint::Cycling),
            )),
            winnow::combinator::alt((
                grammar::kw("reinforce").value(KeywordDispatchHint::Reinforce),
                grammar::kw("equip").value(KeywordDispatchHint::Equip),
                grammar::kw("kicker").value(KeywordDispatchHint::Kicker),
                grammar::kw("flashback").value(KeywordDispatchHint::Flashback),
                grammar::kw("harmonize").value(KeywordDispatchHint::Harmonize),
                grammar::kw("multikicker").value(KeywordDispatchHint::Multikicker),
                grammar::kw("entwine").value(KeywordDispatchHint::Entwine),
                grammar::kw("offspring").value(KeywordDispatchHint::Offspring),
            )),
            winnow::combinator::alt((
                grammar::kw("madness").value(KeywordDispatchHint::Madness),
                grammar::kw("escape").value(KeywordDispatchHint::Escape),
                grammar::kw("morph").value(KeywordDispatchHint::MorphFamily),
                grammar::kw("megamorph").value(KeywordDispatchHint::MorphFamily),
                grammar::kw("squad").value(KeywordDispatchHint::Squad),
                grammar::kw("transmute").value(KeywordDispatchHint::Transmute),
                grammar::phrase(&["cast", "this", "spell", "only"])
                    .value(KeywordDispatchHint::CastThisSpellOnly),
                grammar::kw("gift").value(KeywordDispatchHint::Gift),
                grammar::kw("warp").value(KeywordDispatchHint::Warp),
            )),
        )),
    )
    .map(|(hint, _)| hint);
    if hinted.is_some() {
        return hinted;
    }

    let word_view = TokenWordView::new(tokens);
    let first = word_view.get(0)?;
    if first == "basic" {
        if word_view.get(1) == Some("landcycling") {
            return Some(KeywordDispatchHint::Cycling);
        }
        return None;
    }
    if str_strip_suffix(first, "cycling").is_some() {
        return Some(KeywordDispatchHint::Cycling);
    }

    None
}
