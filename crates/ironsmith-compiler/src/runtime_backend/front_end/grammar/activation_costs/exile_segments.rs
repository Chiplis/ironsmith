use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::color::ColorSet;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::super::{leaf, primitives};
use super::{
    ActivationCostSegmentCst, parse_activation_choice_prefix_tokens,
    parse_activation_exile_filter_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenSpan {
    first: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExileCostShape {
    Targeted,
    Source,
    TopLibrary(u32),
    FromZone {
        subject: TokenSpan,
        zone: Zone,
    },
    NamedArtifacts {
        prefix: TokenSpan,
        names_first: usize,
    },
    Generic {
        subject_first: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExileHandCardShape {
    count: u32,
    color_filter: Option<ColorSet>,
}

pub(crate) fn parse_exile_segment_tokens(
    tokens: &[OwnedLexToken],
    contextual_source_reference: impl Fn(&[&str]) -> bool,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    if let Some(compound) = parse_source_and_chosen_exile(tokens)? {
        return Ok(compound);
    }
    let shape = primitives::parse_all(tokens, parse_exile_cost_shape_lexed, "exile-cost")
        .map_err(|_| unsupported(tokens, "exile"))?;
    match shape {
        ExileCostShape::Targeted => Err(CardTextError::ParseError(
            "unsupported targeted exile cost segment".to_string(),
        )),
        ExileCostShape::Source => Ok(ActivationCostSegmentCst::ExileSelf),
        ExileCostShape::TopLibrary(count) => {
            Ok(ActivationCostSegmentCst::ExileTopLibrary { count })
        }
        ExileCostShape::FromZone { subject, zone } => {
            let subject_tokens = &tokens[subject.first..subject.end];
            let view = primitives::TokenWordView::new(subject_tokens);
            let subject_words = view.word_refs();
            if contextual_source_reference(&subject_words) {
                return Ok(if zone == Zone::Graveyard {
                    ActivationCostSegmentCst::ExileSelfFromGraveyard
                } else {
                    ActivationCostSegmentCst::ExileSelf
                });
            }
            if zone == Zone::Hand
                && let Ok(hand) = primitives::parse_all(
                    subject_tokens,
                    parse_exile_hand_card_lexed,
                    "exile-hand-card",
                )
            {
                return Ok(ActivationCostSegmentCst::ExileFromHand {
                    count: hand.count,
                    color_filter: hand.color_filter,
                });
            }
            let (subject_tokens, top_only) = strip_top_only_prefix(subject_tokens);
            if top_only && zone != Zone::Graveyard {
                return Err(unsupported(tokens, "ordered-exile-source-zone"));
            }
            let choice = parse_activation_choice_prefix_tokens(subject_tokens)
                .ok_or_else(|| unsupported(subject_tokens, "exile-zone-selector"))?;
            let mut filter = parse_activation_exile_filter_tokens(choice.rest)?;
            match zone {
                Zone::Hand | Zone::Graveyard => {
                    filter.zone = Some(zone);
                    if filter.owner.is_none() {
                        filter.owner = Some(PlayerFilter::You);
                    }
                }
                _ => return Err(unsupported(tokens, "exile-source-zone")),
            }
            Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count: choice.count,
                filter,
                top_only,
                turn_face_up: false,
            })
        }
        ExileCostShape::NamedArtifacts {
            prefix,
            names_first,
        } => {
            let prefix_words = primitives::TokenWordView::new(&tokens[prefix.first..prefix.end]);
            if matches!(prefix_words.word_refs().as_slice(), ["the", "top"]) {
                return parse_generic_exile(&tokens[1..]);
            }
            let names = split_named_artifacts(&tokens[names_first..]);
            if names.len() < 2 {
                return parse_generic_exile(&tokens[1..]);
            }
            Ok(ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names })
        }
        ExileCostShape::Generic { subject_first } => parse_generic_exile(&tokens[subject_first..]),
    }
}

fn parse_source_and_chosen_exile(
    tokens: &[OwnedLexToken],
) -> Result<Option<ActivationCostSegmentCst>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("exile")) {
        return Ok(None);
    }
    let body = &tokens[1..];
    let Some(and_idx) = body.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let source_tokens = &body[..and_idx];
    let source_words = primitives::TokenWordView::new(source_tokens).word_refs();
    let Some(source_surface) =
        crate::runtime_backend::front_end::shared::util::this_source_surface_for_words(
            &source_words,
        )
    else {
        return Ok(None);
    };
    let chosen_tokens = &body[and_idx + 1..];
    let Some(choice) = parse_activation_choice_prefix_tokens(chosen_tokens) else {
        return Ok(None);
    };
    // This route is deliberately limited to a fixed, nonzero second set.
    // Optional/dynamic conjunctions need different payment legality.
    if choice.count.min == 0 || choice.count.max != Some(choice.count.min) || choice.count.dynamic_x
    {
        return Ok(None);
    }
    let filter = parse_activation_exile_filter_tokens(choice.rest)?;
    if !filter.other {
        return Ok(None);
    }
    let mut source_filter = crate::target::ObjectFilter::source_with_surface(source_surface);
    source_filter.zone = Some(Zone::Battlefield);
    Ok(Some(ActivationCostSegmentCst::ExileSourceAndChosen {
        source_filter,
        choice_count: choice.count,
        filter,
    }))
}

fn parse_generic_exile(
    subject_tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    // `face up` is the manner of exile, not a second face-state adjective on
    // the selected permanent. It is valid here only after an explicitly
    // face-down subject; otherwise leave the phrase to the ordinary filter
    // parser rather than silently changing its meaning.
    let (subject_tokens, turn_face_up) = if subject_tokens.len() >= 3
        && subject_tokens[subject_tokens.len() - 2].is_word("face")
        && subject_tokens[subject_tokens.len() - 1].is_word("up")
        && subject_tokens[..subject_tokens.len() - 2]
            .iter()
            .any(|token| token.is_word("face-down"))
    {
        (&subject_tokens[..subject_tokens.len() - 2], true)
    } else {
        (subject_tokens, false)
    };
    let (subject_tokens, top_only) = strip_top_only_prefix(subject_tokens);
    let choice = parse_activation_choice_prefix_tokens(subject_tokens)
        .ok_or_else(|| unsupported(subject_tokens, "exile-selector"))?;
    let filter = parse_activation_exile_filter_tokens(choice.rest)?;
    if top_only && filter.zone != Some(Zone::Graveyard) {
        return Err(unsupported(subject_tokens, "ordered-exile-source-zone"));
    }
    Ok(ActivationCostSegmentCst::ExileChosen {
        choice_count: choice.count,
        filter,
        top_only,
        turn_face_up,
    })
}

fn strip_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    primitives::parse_prefix(tokens, primitives::phrase(&["the", "top"]).void())
        .map(|(_, rest)| (rest, true))
        .unwrap_or((tokens, false))
}

fn split_named_artifacts(tokens: &[OwnedLexToken]) -> Vec<String> {
    let view = primitives::TokenWordView::new(tokens);
    let mut names = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in view.word_refs() {
        if word == "and" {
            if !current.is_empty() {
                names.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(word);
        }
    }
    if !current.is_empty() {
        names.push(current.join(" "));
    }
    names
}

fn unsupported(tokens: &[OwnedLexToken], label: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "rewrite {label} parser does not yet support '{}'",
        render_token_slice(tokens).trim().to_ascii_lowercase()
    ))
}

fn parse_exile_cost_shape_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ExileCostShape> {
    let initial_len = input.len();
    primitives::kw("exile").parse_next(input)?;
    alt((
        parse_targeted_exile,
        parse_exile_source,
        parse_exile_top_library,
        move |input: &mut LexStream<'a>| parse_exile_from_zone(input, initial_len),
        move |input: &mut LexStream<'a>| parse_named_artifacts(input, initial_len),
        move |input: &mut LexStream<'a>| parse_generic_shape(input, initial_len),
    ))
    .parse_next(input)
}

fn parse_targeted_exile<'a>(input: &mut LexStream<'a>) -> WResult<ExileCostShape> {
    primitives::kw("target").parse_next(input)?;
    let _: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    Ok(ExileCostShape::Targeted)
}

fn parse_exile_source<'a>(input: &mut LexStream<'a>) -> WResult<ExileCostShape> {
    primitives::kw("this").parse_next(input)?;
    opt(alt((
        primitives::kw("card"),
        primitives::kw("spell"),
        primitives::kw("permanent"),
        primitives::kw("creature"),
        primitives::kw("artifact"),
        primitives::kw("enchantment"),
        primitives::kw("land"),
        primitives::kw("aura"),
        primitives::kw("vehicle"),
    )))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ExileCostShape::Source)
}

fn parse_exile_top_library<'a>(input: &mut LexStream<'a>) -> WResult<ExileCostShape> {
    primitives::phrase(&["the", "top"]).parse_next(input)?;
    let mut count_probe = input.clone();
    let count = if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut count_probe)
    {
        *input = count_probe;
        count
    } else {
        1
    };
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    primitives::phrase(&["of", "your", "library"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ExileCostShape::TopLibrary(count))
}

fn parse_exile_from_zone<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<ExileCostShape> {
    let first = initial_len.saturating_sub(input.len());
    let mut end = first;
    loop {
        let mut hand = input.clone();
        if primitives::phrase(&["from", "your", "hand"])
            .parse_next(&mut hand)
            .is_ok()
            && hand.peek_token().is_none()
        {
            if end == first {
                return Err(primitives::backtrack_err(
                    "exile subject",
                    "nonempty subject",
                ));
            }
            *input = hand;
            return Ok(ExileCostShape::FromZone {
                subject: TokenSpan { first, end },
                zone: Zone::Hand,
            });
        }
        let mut graveyard = input.clone();
        if primitives::phrase(&["from", "your", "graveyard"])
            .parse_next(&mut graveyard)
            .is_ok()
            && graveyard.peek_token().is_none()
        {
            if end == first {
                return Err(primitives::backtrack_err(
                    "exile subject",
                    "nonempty subject",
                ));
            }
            *input = graveyard;
            return Ok(ExileCostShape::FromZone {
                subject: TokenSpan { first, end },
                zone: Zone::Graveyard,
            });
        }
        any.parse_next(input)?;
        end += 1;
    }
}

fn parse_named_artifacts<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<ExileCostShape> {
    let prefix_first = initial_len.saturating_sub(input.len());
    let mut prefix_end = prefix_first;
    loop {
        let mut marker = input.clone();
        if alt((
            primitives::phrase(&["and", "artifacts", "you", "control", "named"]),
            primitives::phrase(&["and", "artifact", "you", "control", "named"]),
        ))
        .parse_next(&mut marker)
        .is_ok()
        {
            if prefix_end == prefix_first {
                return Err(primitives::backtrack_err(
                    "named-artifact exile source",
                    "source before named artifacts",
                ));
            }
            let names_first = initial_len.saturating_sub(marker.len());
            let names: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(&mut marker)?;
            if names.is_empty() {
                return Err(primitives::backtrack_err(
                    "named artifacts",
                    "two or more artifact names",
                ));
            }
            *input = marker;
            return Ok(ExileCostShape::NamedArtifacts {
                prefix: TokenSpan {
                    first: prefix_first,
                    end: prefix_end,
                },
                names_first,
            });
        }
        any.parse_next(input)?;
        prefix_end += 1;
    }
}

fn parse_generic_shape<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<ExileCostShape> {
    let subject_first = initial_len.saturating_sub(input.len());
    let subject: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(input)?;
    if subject.is_empty() {
        return Err(primitives::backtrack_err(
            "exile selector",
            "nonempty selector",
        ));
    }
    Ok(ExileCostShape::Generic { subject_first })
}

fn parse_exile_hand_card_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ExileHandCardShape> {
    let mut count_probe = input.clone();
    let count = if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut count_probe)
    {
        *input = count_probe;
        count
    } else {
        1
    };
    loop {
        let mut article = input.clone();
        if alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        ))
        .parse_next(&mut article)
        .is_err()
        {
            break;
        }
        *input = article;
    }
    let mut color_probe = input.clone();
    let color_filter = if let Ok(word) = primitives::word_parser_text.parse_next(&mut color_probe) {
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            *input = color_probe;
            Some(color)
        } else {
            None
        }
    } else {
        None
    };
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ExileHandCardShape {
        count,
        color_filter,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn exile_segments_preserve_zone_top_and_named_shapes() {
        let top = lex_line("exile the top three cards of your library", 0).unwrap();
        assert_eq!(
            parse_exile_segment_tokens(&top, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileTopLibrary { count: 3 }
        );
        let hand = lex_line("exile a red card from your hand", 0).unwrap();
        assert_eq!(
            parse_exile_segment_tokens(&hand, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileFromHand {
                count: 1,
                color_filter: Some(ColorSet::RED),
            }
        );
        let named = lex_line(
            "exile this card and artifacts you control named foo and bar",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_exile_segment_tokens(&named, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileSelfAndNamedArtifacts {
                names: vec!["foo".to_string(), "bar".to_string()],
            }
        );

        let top_creature = lex_line("exile the top creature card of your graveyard", 0).unwrap();
        assert!(matches!(
            parse_exile_segment_tokens(&top_creature, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter,
                top_only: true,
                turn_face_up: false,
            } if choice_count == crate::effect::ChoiceCount::exactly(1)
                && filter.zone == Some(Zone::Graveyard)
                && filter.card_types == [crate::types::CardType::Creature]
        ));

        let face_up = lex_line("exile a face-down permanent you control face up", 0).unwrap();
        assert!(matches!(
            parse_exile_segment_tokens(&face_up, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter,
                top_only: false,
                turn_face_up: true,
            } if choice_count == crate::effect::ChoiceCount::exactly(1)
                && filter.face_down == Some(true)
                && filter.controller == Some(PlayerFilter::You)
        ));

        let compound = lex_line(
            "exile this Vehicle and four other artifact creatures and/or Vehicles you control",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_exile_segment_tokens(&compound, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileSourceAndChosen {
                source_filter,
                choice_count,
                filter,
            } if source_filter.source
                && matches!(
                    source_filter.source_surface,
                    Some(crate::target::SourceReferenceSurface::ThisPermanentType(ref text))
                        if text == "this Vehicle"
                )
                && choice_count == crate::effect::ChoiceCount::exactly(4)
                && filter.other
                && filter.controller == Some(PlayerFilter::You)
                && filter.card_types.contains(&crate::types::CardType::Artifact)
                && filter.card_types.contains(&crate::types::CardType::Creature)
                && filter.subtypes.contains(&crate::types::Subtype::Vehicle)
        ));

        let ordinary_face_up = lex_line("exile a face-up permanent you control", 0).unwrap();
        assert!(matches!(
            parse_exile_segment_tokens(&ordinary_face_up, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileChosen {
                filter,
                turn_face_up: false,
                ..
            } if filter.face_down == Some(false)
        ));

        let ordinary = lex_line("exile a creature card from your graveyard", 0).unwrap();
        assert!(matches!(
            parse_exile_segment_tokens(&ordinary, |_| false).unwrap(),
            ActivationCostSegmentCst::ExileChosen {
                top_only: false,
                ..
            }
        ));
    }
}
