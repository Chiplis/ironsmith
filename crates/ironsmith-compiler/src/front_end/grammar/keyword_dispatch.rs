use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

mod special_forms;
pub use special_forms::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordDispatchHint {
    AdditionalCostFamily,
    AlternativeOrExertFamily,
    Bestow,
    Blitz,
    Bargain,
    Buyback,
    Channel,
    Craft,
    Cycling,
    Reinforce,
    Equip,
    Reconfigure,
    Kicker,
    Flashback,
    Harmonize,
    Retrace,
    Multikicker,
    Replicate,
    Entwine,
    Escalate,
    Eternalize,
    Evoke,
    Epic,
    Offspring,
    Madness,
    Escape,
    MorphFamily,
    Mutate,
    Squad,
    Splice,
    Transmute,
    Transfigure,
    CastThisSpellOnly,
    Gift,
    Warp,
    Exploit,
}

impl KeywordDispatchHint {
    pub const fn head_words(self) -> &'static [&'static str] {
        match self {
            Self::AdditionalCostFamily => &["as"],
            Self::AlternativeOrExertFamily => &[
                "you",
                "if",
                "prowl",
                "sneak",
                "aftermath",
                "encore",
                "jump-start",
                "jump",
            ],
            Self::Bestow => &["bestow"],
            Self::Blitz => &["blitz"],
            Self::Bargain => &["bargain"],
            Self::Buyback => &["buyback"],
            Self::Channel => &["channel"],
            Self::Craft => &["craft"],
            Self::Cycling => &["cycling", "basic"],
            Self::Reinforce => &["reinforce"],
            Self::Equip => &["equip"],
            Self::Reconfigure => &["reconfigure"],
            Self::Kicker => &["kicker"],
            Self::Flashback => &["flashback"],
            Self::Harmonize => &["harmonize"],
            Self::Retrace => &["retrace"],
            Self::Multikicker => &["multikicker"],
            Self::Replicate => &["replicate"],
            Self::Entwine => &["entwine"],
            Self::Escalate => &["escalate"],
            Self::Eternalize => &["eternalize"],
            Self::Evoke => &["evoke"],
            Self::Epic => &["epic"],
            Self::Offspring => &["offspring"],
            Self::Madness => &["madness"],
            Self::Escape => &["escape"],
            Self::MorphFamily => &["morph", "megamorph", "disguise"],
            Self::Mutate => &["mutate"],
            Self::Squad => &["squad"],
            Self::Splice => &["splice"],
            Self::Transmute => &["transmute"],
            Self::Transfigure => &["transfigure"],
            Self::CastThisSpellOnly => &["cast"],
            Self::Gift => &["gift"],
            Self::Warp => &["warp"],
            Self::Exploit => &["exploit"],
        }
    }
}

pub fn parse_keyword_dispatch_hint_tokens(tokens: &[OwnedLexToken]) -> Option<KeywordDispatchHint> {
    if let Some((hint, _)) = primitives::parse_prefix(tokens, parse_keyword_dispatch_hint_lexed) {
        return Some(hint);
    }

    let words = primitives::TokenWordView::new(tokens).word_refs();
    let first = words.first().copied()?;
    let fallback = parse_keyword_fallback_kind_tokens(tokens);
    if fallback == Some(KeywordFallbackKind::BasicLandcycling) {
        return Some(KeywordDispatchHint::Cycling);
    }
    if first == "basic" {
        return None;
    }
    if super::shared_util::reference_shapes::cycling_keyword_root(first).is_some() {
        return Some(KeywordDispatchHint::Cycling);
    }
    if matches!(
        fallback,
        Some(
            KeywordFallbackKind::Aftermath
                | KeywordFallbackKind::Encore
                | KeywordFallbackKind::JumpStart
        )
    ) {
        return Some(KeywordDispatchHint::AlternativeOrExertFamily);
    }
    None
}

fn parse_keyword_dispatch_hint_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordDispatchHint> {
    alt((
        alt((
            primitives::phrase(&[
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
            primitives::kw("you").value(KeywordDispatchHint::AlternativeOrExertFamily),
            primitives::kw("if").value(KeywordDispatchHint::AlternativeOrExertFamily),
            primitives::kw("bestow").value(KeywordDispatchHint::Bestow),
            primitives::kw("blitz").value(KeywordDispatchHint::Blitz),
            primitives::kw("bargain").value(KeywordDispatchHint::Bargain),
            primitives::kw("buyback").value(KeywordDispatchHint::Buyback),
            primitives::kw("channel").value(KeywordDispatchHint::Channel),
            primitives::kw("cycling").value(KeywordDispatchHint::Cycling),
        )),
        alt((
            primitives::kw("reinforce").value(KeywordDispatchHint::Reinforce),
            primitives::kw("equip").value(KeywordDispatchHint::Equip),
            primitives::kw("kicker").value(KeywordDispatchHint::Kicker),
            primitives::kw("flashback").value(KeywordDispatchHint::Flashback),
            primitives::kw("harmonize").value(KeywordDispatchHint::Harmonize),
            primitives::kw("multikicker").value(KeywordDispatchHint::Multikicker),
            primitives::kw("replicate").value(KeywordDispatchHint::Replicate),
            primitives::kw("entwine").value(KeywordDispatchHint::Entwine),
            alt((
                primitives::kw("offspring").value(KeywordDispatchHint::Offspring),
                primitives::kw("splice").value(KeywordDispatchHint::Splice),
            )),
        )),
        alt((
            primitives::kw("retrace").value(KeywordDispatchHint::Retrace),
            primitives::kw("madness").value(KeywordDispatchHint::Madness),
            primitives::kw("escape").value(KeywordDispatchHint::Escape),
            alt((
                primitives::kw("morph").value(KeywordDispatchHint::MorphFamily),
                primitives::kw("megamorph").value(KeywordDispatchHint::MorphFamily),
                primitives::kw("disguise").value(KeywordDispatchHint::MorphFamily),
                primitives::kw("mutate").value(KeywordDispatchHint::Mutate),
            )),
            primitives::kw("squad").value(KeywordDispatchHint::Squad),
            alt((
                primitives::kw("transmute").value(KeywordDispatchHint::Transmute),
                primitives::kw("transfigure").value(KeywordDispatchHint::Transfigure),
            )),
            primitives::kw("reconfigure").value(KeywordDispatchHint::Reconfigure),
            primitives::kw("eternalize").value(KeywordDispatchHint::Eternalize),
            primitives::phrase(&["cast", "this", "spell", "only"])
                .value(KeywordDispatchHint::CastThisSpellOnly),
        )),
        alt((
            primitives::kw("gift").value(KeywordDispatchHint::Gift),
            primitives::kw("warp").value(KeywordDispatchHint::Warp),
            primitives::kw("prowl").value(KeywordDispatchHint::AlternativeOrExertFamily),
            primitives::kw("sneak").value(KeywordDispatchHint::AlternativeOrExertFamily),
            primitives::kw("escalate").value(KeywordDispatchHint::Escalate),
            primitives::kw("evoke").value(KeywordDispatchHint::Evoke),
            primitives::kw("epic").value(KeywordDispatchHint::Epic),
            primitives::kw("craft").value(KeywordDispatchHint::Craft),
            primitives::kw("exploit").value(KeywordDispatchHint::Exploit),
        )),
    ))
    .parse_next(input)
}

#[cfg(test)]
#[path = "keyword_dispatch_inline_tests.rs"]
mod tests;
