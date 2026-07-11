use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

mod special_forms;
pub(crate) use special_forms::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordDispatchHint {
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
    CastThisSpellOnly,
    Gift,
    Warp,
    Exploit,
}

pub(crate) fn parse_keyword_dispatch_hint_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordDispatchHint> {
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
            primitives::kw("transmute").value(KeywordDispatchHint::Transmute),
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
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn fallback_dispatch_kind_accepts_surface_variants() {
        for (text, expected) in [
            ("Aftermath", KeywordFallbackKind::Aftermath),
            (
                "Basic landcycling {2}",
                KeywordFallbackKind::BasicLandcycling,
            ),
            ("Encore {3}{B}", KeywordFallbackKind::Encore),
            ("Jump-start", KeywordFallbackKind::JumpStart),
            ("Jump start", KeywordFallbackKind::JumpStart),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(parse_keyword_fallback_kind_tokens(&tokens), Some(expected));
        }
    }

    #[test]
    fn full_dispatch_hint_parser_owns_direct_and_fallback_recognition() {
        for (text, expected) in [
            ("Buyback {2}", KeywordDispatchHint::Buyback),
            ("Basic landcycling {2}", KeywordDispatchHint::Cycling),
            ("Islandcycling {2}", KeywordDispatchHint::Cycling),
            ("Jump-start", KeywordDispatchHint::AlternativeOrExertFamily),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(parse_keyword_dispatch_hint_tokens(&tokens), Some(expected));
        }
    }

    #[test]
    fn keyword_prefix_and_special_forms_are_typed() {
        let prefix = lex_line("Freerunning {1}{B}", 0).unwrap();
        assert_eq!(
            parse_keyword_prefix_shape_tokens(&prefix),
            Some(KeywordPrefixShape::Freerunning)
        );

        let blitz = lex_line(
            "You may cast this card from your graveyard using its blitz ability",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_special_form_shape_tokens(&blitz),
            Some(KeywordSpecialFormShape::BlitzFromGraveyard)
        );

        let sneak = lex_line("It enters tapped and attacking", 0).unwrap();
        assert_eq!(
            parse_keyword_special_form_shape_tokens(&sneak),
            Some(KeywordSpecialFormShape::PermanentSneak)
        );

        let exert = lex_line(
            "If this creature hasn't been exerted this turn, you may exert it as it attacks.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_special_form_shape_tokens(&exert),
            Some(KeywordSpecialFormShape::ExertAttack)
        );
    }
}
