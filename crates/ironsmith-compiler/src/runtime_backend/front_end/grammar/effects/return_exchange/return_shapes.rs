use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use crate::runtime_backend::front_end::grammar::permission_shapes;
use crate::runtime_backend::front_end::shared::util::parse_subtype_flexible;
use crate::runtime_backend::lexer::TokenWordView;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnZoneShape {
    Hand,
    Battlefield,
    Graveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnControllerShape {
    Preserve,
    You,
    Owner,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReturnTimingShape {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReturnDestinationShape {
    pub(crate) zone: ReturnZoneShape,
    pub(crate) tapped: bool,
    pub(crate) transformed: bool,
    pub(crate) converted: bool,
    pub(crate) controller: ReturnControllerShape,
    pub(crate) timing: Option<ReturnTimingShape>,
    pub(crate) has_unparsed_timing_words: bool,
    pub(crate) attached_to_tokens: Option<Vec<OwnedLexToken>>,
    pub(crate) excluded_subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReturnTargetShape {
    PairedSourceAndExiled {
        source_subtype: Option<Subtype>,
    },
    UntargetedExiledCards {
        filter_tokens: Vec<OwnedLexToken>,
    },
    All {
        raw_filter_tokens: Vec<OwnedLexToken>,
        filter_tokens: Vec<OwnedLexToken>,
        chosen_this_way_excluded: Option<bool>,
        chosen_creature_type: bool,
        excluded_chosen_creature_type: bool,
        discarded_or_cycled_this_turn_by: Option<PlayerFilter>,
        unsupported_qualifier: bool,
    },
    Singular {
        target_tokens: Vec<OwnedLexToken>,
        source_from_graveyard_tokens: Option<Vec<OwnedLexToken>>,
        dynamic_count: bool,
        back_reference: bool,
    },
    MultiTargetUnsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReturnClauseShape {
    pub(crate) target: ReturnTargetShape,
    pub(crate) destination: ReturnDestinationShape,
    pub(crate) random: bool,
    pub(crate) has_unless: bool,
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn token_is(token: &OwnedLexToken, expected: &'static str) -> bool {
    primitives::parse_all(
        std::slice::from_ref(token),
        primitives::kw(expected).void(),
        "return surface word",
    )
    .is_ok()
}

fn zone_word<'a>(input: &mut LexStream<'a>) -> WResult<ReturnZoneShape> {
    alt((
        alt((primitives::kw("hand"), primitives::kw("hands"))).value(ReturnZoneShape::Hand),
        primitives::kw("battlefield").value(ReturnZoneShape::Battlefield),
        alt((primitives::kw("graveyard"), primitives::kw("graveyards")))
            .value(ReturnZoneShape::Graveyard),
    ))
    .parse_next(input)
}

fn first_zone(tokens: &[OwnedLexToken]) -> Option<(usize, ReturnZoneShape)> {
    let mut input = LexStream::new(tokens);
    let (zone, taken) = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), zone_word)
        .map(|((), zone)| zone)
        .with_taken()
        .parse_next(&mut input)
        .ok()?;
    Some((taken.len().checked_sub(1)?, zone))
}

fn normalize_destination_first(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("to"))?;
    let destination_start = tokens.len().checked_sub(rest.len())?;
    let (zone_offset, _) = first_zone(rest)?;
    let mut split = destination_start + zone_offset + 1;
    if tokens
        .get(split)
        .is_some_and(|token| token_is(token, "under"))
    {
        let control_offset = first_word_offset(&tokens[split + 1..], "control")?;
        split += control_offset + 2;
    }
    while tokens
        .get(split)
        .is_some_and(|token| token_is(token, "tapped"))
    {
        split += 1;
    }
    let target = trim_lexed_commas(tokens.get(split..)?);
    let destination = trim_lexed_commas(tokens.get(..split)?);
    if target.is_empty() || destination.is_empty() {
        return None;
    }
    let mut normalized = target.to_vec();
    normalized.extend_from_slice(destination);
    Some(normalized)
}

fn first_word_offset(tokens: &[OwnedLexToken], expected: &'static str) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let (_, taken) = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw(expected))
        .with_taken()
        .parse_next(&mut input)
        .ok()?;
    taken.len().checked_sub(1)
}

fn last_destination_split(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if !token_is(&tokens[idx], "to") {
            continue;
        }
        if first_zone(tokens.get(idx + 1..)?).is_some() {
            return Some(idx);
        }
    }
    None
}

fn remove_at_random(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let mut input = LexStream::new(tokens);
    let found =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::phrase(&["at", "random"]))
            .map(|((), ())| ())
            .with_taken()
            .parse_next(&mut input)
            .ok();
    let Some(((), taken)) = found else {
        return (tokens.to_vec(), false);
    };
    let marker_start = taken.len() - 2;
    let mut cleaned = tokens.get(..marker_start).unwrap_or_default().to_vec();
    cleaned.extend_from_slice(tokens.get(taken.len()..).unwrap_or_default());
    (cleaned, true)
}

fn parse_return_timing_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ReturnTimingShape> {
    primitives::kw("at").parse_next(input)?;
    let mut end_combat_probe = input.clone();
    if opt(primitives::kw("the"))
        .parse_next(&mut end_combat_probe)
        .is_ok()
        && primitives::phrase(&["end", "of", "combat"])
            .parse_next(&mut end_combat_probe)
            .is_ok()
        && primitives::sentence_end()
            .parse_next(&mut end_combat_probe)
            .is_ok()
    {
        *input = end_combat_probe;
        return Ok(ReturnTimingShape::EndOfCombat);
    }
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("beginning").parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let player_you = opt(primitives::kw("your")).parse_next(input)?.is_some();
    primitives::kw("next").parse_next(input)?;
    let timing = alt((
        primitives::phrase(&["end", "step"]).value(if player_you {
            ReturnTimingShape::NextEndStep(PlayerFilter::You)
        } else {
            ReturnTimingShape::NextEndStep(PlayerFilter::Any)
        }),
        primitives::kw("upkeep").value(if player_you {
            ReturnTimingShape::NextUpkeep(PlayerAst::You)
        } else {
            ReturnTimingShape::NextUpkeep(PlayerAst::Any)
        }),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(timing)
}

pub(crate) fn parse_return_timing_words_shape(words: &[&str]) -> Option<ReturnTimingShape> {
    let tokens = words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    primitives::parse_all(&tokens, parse_return_timing_lexed, "return timing words").ok()
}

fn split_timing(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<ReturnTimingShape>) {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if token_is(&tokens[idx], "at")
            && let Ok(timing) =
                primitives::parse_all(&tokens[idx..], parse_return_timing_lexed, "return timing")
        {
            return (&tokens[..idx], Some(timing));
        }
        idx += 1;
    }
    (tokens, None)
}

fn split_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let mut input = LexStream::new(tokens);
    let (_, taken) =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::phrase(phrase))
            .with_taken()
            .parse_next(&mut input)
            .ok()?;
    let marker_start = taken.len().checked_sub(phrase.len())?;
    Some((&tokens[..marker_start], &tokens[taken.len()..]))
}

fn parse_destination(tokens: &[OwnedLexToken]) -> Option<ReturnDestinationShape> {
    let (without_timing, timing) = split_timing(tokens);
    let (without_attachment, attached_to_tokens) =
        if let Some((head, target)) = split_phrase(without_timing, &["attached", "to"]) {
            (head, Some(trim_lexed_commas(target).to_vec()))
        } else {
            (without_timing, None)
        };
    let (destination_head, exception_tokens) =
        if let Some((head, exceptions)) = split_phrase(without_attachment, &["except", "for"]) {
            (head, Some(exceptions))
        } else {
            (without_attachment, None)
        };
    let mut excluded_subtypes = Vec::new();
    for token in exception_tokens.unwrap_or_default() {
        if token_is(token, "and") || token_is(token, "or") {
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        let subtype = parse_subtype_flexible(word)?;
        if excluded_subtypes
            .iter()
            .all(|existing| existing != &subtype)
        {
            excluded_subtypes.push(subtype);
        }
    }
    if exception_tokens.is_some() && excluded_subtypes.is_empty() {
        return None;
    }
    let (_, zone) = first_zone(destination_head)?;
    let tapped = marker_anywhere(destination_head, primitives::kw("tapped"));
    let transformed = marker_anywhere(tokens, primitives::kw("transformed"));
    let converted = marker_anywhere(tokens, primitives::kw("converted"));
    let controller = if marker_anywhere(
        destination_head,
        primitives::phrase(&["under", "your", "control"]),
    ) {
        ReturnControllerShape::You
    } else if marker_anywhere(
        destination_head,
        alt((
            primitives::kw("owner"),
            primitives::kw("owners"),
            primitives::kw("owner's"),
            primitives::kw("owners'"),
        )),
    ) {
        ReturnControllerShape::Owner
    } else {
        ReturnControllerShape::Preserve
    };
    let has_unparsed_timing_words = timing.is_none()
        && (marker_anywhere(tokens, primitives::kw("beginning"))
            || marker_anywhere(tokens, primitives::kw("upkeep"))
            || marker_anywhere(tokens, primitives::phrase(&["end", "of", "combat"]))
            || (marker_anywhere(tokens, primitives::kw("end"))
                && (marker_anywhere(tokens, primitives::kw("next"))
                    || marker_anywhere(tokens, primitives::kw("step")))));
    Some(ReturnDestinationShape {
        zone,
        tapped,
        transformed,
        converted,
        controller,
        timing,
        has_unparsed_timing_words,
        attached_to_tokens,
        excluded_subtypes,
    })
}

fn exact_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases.iter().any(|phrase| {
        primitives::parse_all(
            tokens,
            (dynamic_phrase(phrase), eof).void(),
            "return exact surface",
        )
        .is_ok()
    })
}

fn dynamic_phrase<'a, 'p>(
    words: &'p [&'p str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        for word in words {
            let expected = *word;
            any.verify(move |token: &&OwnedLexToken| token.is_word(expected))
                .void()
                .parse_next(input)?;
        }
        Ok(())
    }
}

fn split_suffix<'a>(
    tokens: &'a [OwnedLexToken],
    alternatives: &[(&[&str], bool)],
) -> Option<(&'a [OwnedLexToken], bool)> {
    for (suffix, flag) in alternatives {
        let words = TokenWordView::new(tokens).word_refs();
        if suffix.len() > words.len() {
            continue;
        }
        let cutoff = words.len() - suffix.len();
        if !permission_shapes::exact_words(&words[cutoff..], suffix) {
            continue;
        }
        let token_cutoff = TokenWordView::new(tokens).token_index_after_words(cutoff)?;
        return Some((trim_lexed_commas(&tokens[..token_cutoff]), *flag));
    }
    None
}

fn paired_source_subtype(tokens: &[OwnedLexToken]) -> Option<Option<Subtype>> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("this"))?;
    Some(
        rest.first()
            .and_then(OwnedLexToken::as_word)
            .and_then(parse_subtype_flexible),
    )
}

fn paired_source_and_exiled(tokens: &[OwnedLexToken]) -> Option<Option<Subtype>> {
    let (left, right) = split_phrase(tokens, &["and"])?;
    let exiled = [
        &["the", "exiled", "card"][..],
        &["the", "exiled", "cards"][..],
        &["exiled", "card"][..],
        &["exiled", "cards"][..],
    ];
    paired_source_subtype(trim_lexed_commas(left))
        .filter(|_| exact_any(trim_lexed_commas(right), &exiled))
        .or_else(|| {
            paired_source_subtype(trim_lexed_commas(right))
                .filter(|_| exact_any(trim_lexed_commas(left), &exiled))
        })
}

pub(crate) fn is_return_back_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_any(
        tokens,
        &[
            &["it"],
            &["them"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "object"],
            &["that", "permanent"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "objects"],
            &["those", "permanents"],
        ],
    )
}

fn starts_multi_target(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        alt((primitives::kw("target"), primitives::kw("targets"))),
    )
    .is_some()
        || {
            let mut input = LexStream::new(tokens);
            leaf::parse_leaf_choice_count_prefix_lexed
                .parse_next(&mut input)
                .is_ok()
                && alt((primitives::kw("target"), primitives::kw("targets")))
                    .parse_next(&mut input)
                    .is_ok()
        }
}

fn classify_target(tokens: &[OwnedLexToken], zone: ReturnZoneShape) -> Option<ReturnTargetShape> {
    if zone == ReturnZoneShape::Hand
        && let Some(source_subtype) = paired_source_and_exiled(tokens)
    {
        return Some(ReturnTargetShape::PairedSourceAndExiled { source_subtype });
    }
    if let Some((_, tail)) = split_phrase(tokens, &["and"])
        && starts_multi_target(trim_lexed_commas(tail))
    {
        return Some(ReturnTargetShape::MultiTargetUnsupported);
    }
    let has_target = marker_anywhere(tokens, primitives::kw("target"));
    let has_exiled_cards = marker_anywhere(tokens, primitives::kw("exiled"))
        && marker_anywhere(tokens, primitives::kw("cards"));
    if !has_target && has_exiled_cards {
        return Some(ReturnTargetShape::UntargetedExiledCards {
            filter_tokens: tokens.to_vec(),
        });
    }

    if let Some((_, rest)) =
        primitives::parse_prefix(tokens, alt((primitives::kw("all"), primitives::kw("each"))))
    {
        let raw_filter_tokens = trim_lexed_commas(rest).to_vec();
        let unsupported_qualifier = marker_anywhere(rest, primitives::kw("dealt"))
            || (marker_anywhere(rest, primitives::kw("without"))
                && marker_anywhere(rest, primitives::kw("counter")));
        let chosen_this_way = [
            (&["not", "chosen", "this", "way"][..], true),
            (&["that", "weren't", "chosen", "this", "way"][..], true),
            (&["that", "werent", "chosen", "this", "way"][..], true),
            (&["that", "were", "not", "chosen", "this", "way"][..], true),
            (&["chosen", "this", "way"][..], false),
            (&["that", "were", "chosen", "this", "way"][..], false),
            (&["that", "was", "chosen", "this", "way"][..], false),
        ];
        let (without_chosen, chosen_this_way_excluded) = split_suffix(rest, &chosen_this_way)
            .map(|(head, excluded)| (head.to_vec(), Some(excluded)))
            .unwrap_or_else(|| (rest.to_vec(), None));
        let chosen_type = [
            (&["of", "the", "chosen", "type"][..], false),
            (&["that", "are", "of", "the", "chosen", "type"][..], false),
            (&["that", "arent", "of", "the", "chosen", "type"][..], true),
            (&["that", "aren't", "of", "the", "chosen", "type"][..], true),
            (
                &["that", "are", "not", "of", "the", "chosen", "type"][..],
                true,
            ),
        ];
        let (without_type, chosen_type_flag) = split_suffix(&without_chosen, &chosen_type)
            .map(|(head, excluded)| (head.to_vec(), Some(excluded)))
            .unwrap_or((without_chosen, None));
        let (filter_tokens, discarded_or_cycled_this_turn_by) =
            match super::parse_cycled_or_discarded_this_turn_filter_tail_tokens(&without_type)
                .ok()
                .flatten()
            {
                Some(tail) => (tail.base_tokens, Some(tail.player_filter)),
                None => (without_type, None),
            };
        return Some(ReturnTargetShape::All {
            raw_filter_tokens,
            filter_tokens: trim_lexed_commas(&filter_tokens).to_vec(),
            chosen_this_way_excluded,
            chosen_creature_type: chosen_type_flag == Some(false),
            excluded_chosen_creature_type: chosen_type_flag == Some(true),
            discarded_or_cycled_this_turn_by,
            unsupported_qualifier,
        });
    }

    let graveyard_tails = [
        (&["from", "your", "graveyard"][..], false),
        (&["from", "its", "owner", "graveyard"][..], false),
        (&["from", "its", "owners", "graveyard"][..], false),
        (&["from", "its", "owner's", "graveyard"][..], false),
        (&["from", "its", "owners'", "graveyard"][..], false),
    ];
    let source_from_graveyard_tokens = if zone == ReturnZoneShape::Battlefield {
        split_suffix(tokens, &graveyard_tails).map(|(head, _)| head.to_vec())
    } else {
        None
    };
    let (target_tokens, dynamic_count) = if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["that", "many"]),
            opt(primitives::kw("of")),
        )
            .void(),
    ) {
        (trim_lexed_commas(rest).to_vec(), true)
    } else {
        (tokens.to_vec(), false)
    };
    Some(ReturnTargetShape::Singular {
        back_reference: is_return_back_reference_shape(&target_tokens),
        target_tokens,
        source_from_graveyard_tokens,
        dynamic_count,
    })
}

pub(crate) fn parse_return_clause_shape(tokens: &[OwnedLexToken]) -> Option<ReturnClauseShape> {
    let normalized;
    let tokens = if primitives::parse_prefix(tokens, primitives::kw("to")).is_some() {
        normalized = normalize_destination_first(tokens)?;
        normalized.as_slice()
    } else {
        tokens
    };
    let has_unless = marker_anywhere(tokens, primitives::kw("unless"));
    let split = last_destination_split(tokens)?;
    let (target_tokens, random) = remove_at_random(trim_lexed_commas(&tokens[..split]));
    let destination = parse_destination(trim_lexed_commas(&tokens[split + 1..]))?;
    let target = classify_target(&target_tokens, destination.zone)?;
    Some(ReturnClauseShape {
        target,
        destination,
        random,
        has_unless,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_return_all_surface_facts() {
        let tokens = lex_line(
            "all creature cards not chosen this way to their owners' hands",
            0,
        )
        .unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert!(matches!(
            shape.target,
            ReturnTargetShape::All {
                chosen_this_way_excluded: Some(true),
                ..
            }
        ));
        assert_eq!(shape.destination.zone, ReturnZoneShape::Hand);
    }

    #[test]
    fn parses_delayed_attached_return_surface() {
        let tokens = lex_line(
            "target Aura to the battlefield attached to it at the beginning of the next end step",
            0,
        )
        .unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert!(shape.destination.attached_to_tokens.is_some());
        assert!(matches!(
            shape.destination.timing,
            Some(ReturnTimingShape::NextEndStep(PlayerFilter::Any))
        ));
    }

    #[test]
    fn normalizes_destination_first_return_surface() {
        let tokens = lex_line("to their owners' hands all creatures", 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert!(matches!(shape.target, ReturnTargetShape::All { .. }));
        assert_eq!(shape.destination.zone, ReturnZoneShape::Hand);
        assert_eq!(shape.destination.controller, ReturnControllerShape::Owner);
    }

    #[test]
    fn preserves_destination_first_control_boundary() {
        let tokens = lex_line("to the battlefield under your control target creature", 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert_eq!(shape.destination.controller, ReturnControllerShape::You);
        let ReturnTargetShape::Singular { target_tokens, .. } = shape.target else {
            panic!("expected singular target");
        };
        assert_eq!(target_tokens.len(), 2);
    }

    #[test]
    fn removes_random_marker_without_truncating_target() {
        let tokens = lex_line("a card exiled with it at random to its owner's hand", 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert!(shape.random);
        let ReturnTargetShape::Singular { target_tokens, .. } = shape.target else {
            panic!("expected singular target");
        };
        assert_eq!(target_tokens.len(), 5);
        assert!(
            target_tokens
                .last()
                .is_some_and(|token| token_is(token, "it"))
        );
    }

    #[test]
    fn preserves_source_subtype_in_paired_source_and_exiled_surface() {
        let tokens = lex_line("this Elf card and exiled cards to their owners' hands", 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert!(matches!(
            shape.target,
            ReturnTargetShape::PairedSourceAndExiled {
                source_subtype: Some(Subtype::Elf),
            }
        ));
    }
}
