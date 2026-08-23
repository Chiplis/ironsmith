use crate::lexer::{OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use crate::target::SourceReferenceSurface;
use crate::types::Subtype;
use crate::util::{source_reference_surface_for_words, this_source_surface_for_words};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, permission_shapes, primitives};

const TAGGED_REFERENCES: &[&[&str]] = &[
    &["it"],
    &["they"],
    &["them"],
    &["that"],
    &["that", "card"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "object"],
    &["those"],
    &["those", "cards"],
    &["those", "creatures"],
    &["those", "permanents"],
    &["those", "objects"],
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BecomeMassTargetKind {
    Creature,
    Land,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BecomeTargetSubjectShape<'a> {
    Mass(BecomeMassTargetKind),
    Tagged,
    FilteredMany(&'a [OwnedLexToken]),
    Source(SourceReferenceSurface),
    Parsed(&'a [OwnedLexToken]),
}

fn exact_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn tagged_reference(words: &[&str]) -> bool {
    if exact_any(words, TAGGED_REFERENCES) {
        return true;
    }
    for prefix in [
        &["each", "of"][..],
        &["all", "of"][..],
        &["each"][..],
        &["all"][..],
    ] {
        if permission_shapes::prefix_words(words, prefix)
            && exact_any(&words[prefix.len()..], TAGGED_REFERENCES)
        {
            return true;
        }
    }
    false
}

fn is_basic_land_type(word: &str) -> bool {
    leaf::parse_leaf_subtype_flexible_complete(word).is_ok_and(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    })
}

pub fn parse_become_target_subject_shape<'a>(
    target_tokens: &'a [OwnedLexToken],
    body_tokens: &[OwnedLexToken],
) -> BecomeTargetSubjectShape<'a> {
    let target_tokens = trim_lexed_commas(target_tokens);
    let target_words = parser_token_word_refs(target_tokens);
    let body_words = parser_token_word_refs(body_tokens);
    if permission_shapes::exact_words(&target_words, &["all"]) {
        let mass_kind =
            if body_words.len() == 1 && leaf::parse_leaf_color_complete(body_words[0]).is_ok() {
                BecomeMassTargetKind::Creature
            } else if body_words.len() == 1 && is_basic_land_type(body_words[0]) {
                BecomeMassTargetKind::Land
            } else {
                BecomeMassTargetKind::Unsupported
            };
        return BecomeTargetSubjectShape::Mass(mass_kind);
    }
    if target_words.is_empty()
        || exact_any(
            &target_words,
            &[
                &["it"],
                &["it's"],
                &["its"],
                &["it’s"],
                &["they"],
                &["them"],
            ],
        )
        || tagged_reference(&target_words)
    {
        return BecomeTargetSubjectShape::Tagged;
    }
    if let Some((_, filter_tokens)) = primitives::parse_prefix(
        target_tokens,
        alt((primitives::kw("all").void(), primitives::kw("each").void())),
    ) && !filter_tokens.is_empty()
    {
        return BecomeTargetSubjectShape::FilteredMany(filter_tokens);
    }
    if let Some(surface) = source_reference_surface_for_words(&target_words)
        .or_else(|| this_source_surface_for_words(&target_words))
    {
        return BecomeTargetSubjectShape::Source(surface);
    }
    BecomeTargetSubjectShape::Parsed(target_tokens)
}

pub fn become_subject_set_quantifier_surface(
    target_tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::SetQuantifierSurface> {
    let words = parser_token_word_refs(trim_lexed_commas(target_tokens));
    match words.first().copied() {
        Some("they" | "theyre" | "they're" | "them") => {
            Some(ironsmith_core::SetQuantifierSurface::They)
        }
        Some("those") => Some(ironsmith_core::SetQuantifierSurface::Those),
        Some("each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
        _ => None,
    }
}

pub fn become_subject_has_life_total(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::contains_tokens(tokens, &["life"])
        && permission_shapes::contains_tokens(tokens, &["total"])
}

pub fn parse_leading_duration_target_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (target_offset, _, _) =
        primitives::find_prefix(tokens, || primitives::kw("target").void())?;
    let target_tokens = trim_lexed_commas(&tokens[target_offset..]);
    (target_tokens.len() > 1).then_some(target_tokens)
}

pub fn aura_subject_prefers_source(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    exact_any(&words, &[&["it"], &["this"], &["this", "creature"]])
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn classifies_typed_target_subjects_and_duration_recovery() {
        let subject = lex("all");
        let body = lex("blue");
        assert!(matches!(
            parse_become_target_subject_shape(&subject, &body),
            BecomeTargetSubjectShape::Mass(BecomeMassTargetKind::Creature)
        ));

        let tagged = lex("each of those creatures");
        assert!(matches!(
            parse_become_target_subject_shape(&tagged, &body),
            BecomeTargetSubjectShape::Tagged
        ));
        assert_eq!(
            become_subject_set_quantifier_surface(&lex("they")),
            Some(ironsmith_core::SetQuantifierSurface::They)
        );
        assert_eq!(become_subject_set_quantifier_surface(&lex("it")), None);

        let duration = lex("until end of turn, target artifact");
        assert_eq!(
            parse_leading_duration_target_tokens(&duration)
                .map(parser_token_word_refs)
                .unwrap(),
            ["target", "artifact"]
        );

        crate::util::with_source_reference_context("Sarkhan, Soul Aflame", || {
            let named_source = lex("Sarkhan");
            assert_eq!(
                parse_become_target_subject_shape(&named_source, &body),
                BecomeTargetSubjectShape::Source(SourceReferenceSurface::ShortName(
                    "Sarkhan".to_string()
                ))
            );
        });
    }
}
