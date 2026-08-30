use super::super::*;

use crate::grammar::leaf;
use crate::grammar::permission_shapes;
use crate::lexer::TokenWordView;
use crate::util::parse_subtype_flexible;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnZoneShape {
    Hand,
    Battlefield,
    Graveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnControllerShape {
    Preserve,
    You,
    Owner,
    ThatPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReturnTimingShape {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnDestinationShape {
    pub zone: ReturnZoneShape,
    pub destination_player_surface: Option<PlayerAst>,
    pub tapped: bool,
    pub attacking: bool,
    pub face_down: bool,
    pub transformed: bool,
    pub converted: bool,
    pub controller: ReturnControllerShape,
    pub timing: Option<ReturnTimingShape>,
    pub has_unparsed_timing_words: bool,
    pub attached_to_tokens: Option<Vec<OwnedLexToken>>,
    pub excluded_subtypes: Vec<Subtype>,
    /// Counters the returned object enters with, as authored after `with`.
    pub entry_counter_tokens: Option<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReturnTargetShape {
    PairedSourceAndExiled {
        source_subtype: Option<Subtype>,
    },
    UntargetedExiledCards {
        filter_tokens: Vec<OwnedLexToken>,
        count: Option<ChoiceCount>,
    },
    All {
        set_quantifier_surface: ironsmith_core::SetQuantifierSurface,
        raw_filter_tokens: Vec<OwnedLexToken>,
        filter_tokens: Vec<OwnedLexToken>,
        chosen_this_way_excluded: Option<bool>,
        chosen_creature_type: bool,
        excluded_chosen_creature_type: bool,
        chosen_type_this_way_surface: bool,
        discarded_or_cycled_this_turn_by: Option<PlayerFilter>,
        unsupported_qualifier: bool,
    },
    Singular {
        target_tokens: Vec<OwnedLexToken>,
        source_from_graveyard_tokens: Option<Vec<OwnedLexToken>>,
        source_from_graveyard_or_exile_tokens: Option<Vec<OwnedLexToken>>,
        dynamic_count: bool,
        back_reference: bool,
        top_only: bool,
    },
    MultiTargetUnsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnClauseShape {
    pub target: ReturnTargetShape,
    pub destination: ReturnDestinationShape,
    pub destination_first: bool,
    pub random: bool,
    pub has_unless: bool,
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

pub fn parse_return_timing_words_shape(words: &[&str]) -> Option<ReturnTimingShape> {
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
    // `... to the battlefield with a finality counter on it` states what the
    // returned object enters with. Split that clause out so the return action
    // can pair it with the counters it names instead of dropping the tail.
    let (tokens, entry_counter_tokens) = match split_phrase(tokens, &["with"]) {
        Some((head, tail))
            if crate::grammar::primitives::find_prefix(tail, || {
                alt((primitives::kw("counter"), primitives::kw("counters"))).void()
            })
            .is_some() =>
        {
            (head, Some(tail.to_vec()))
        }
        _ => (tokens, None),
    };
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
    let destination_player_surface = if marker_anywhere(
        destination_head,
        alt((
            primitives::kw("owner"),
            primitives::kw("owners"),
            primitives::kw("owner's"),
            primitives::kw("owners'"),
        )),
    ) {
        None
    } else if marker_anywhere(
        destination_head,
        alt((primitives::kw("your"), primitives::kw("you"))),
    ) {
        Some(PlayerAst::You)
    } else if marker_anywhere(
        destination_head,
        alt((
            primitives::kw("their").void(),
            primitives::phrase(&["that", "player"]),
            primitives::phrase(&["that", "players"]),
        )),
    ) {
        Some(PlayerAst::That)
    } else {
        None
    };
    let tapped = marker_anywhere(destination_head, primitives::kw("tapped"));
    let attacking = marker_anywhere(destination_head, primitives::kw("attacking"));
    let face_down = marker_anywhere(
        destination_head,
        alt((
            primitives::phrase(&["face", "down"]).void(),
            primitives::kw("face-down").void(),
            primitives::kw("facedown").void(),
        )),
    );
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
            primitives::phrase(&["under", "that", "player", "control"]).void(),
            primitives::phrase(&["under", "that", "players", "control"]).void(),
            primitives::phrase(&["under", "that", "player's", "control"]).void(),
        )),
    ) {
        ReturnControllerShape::ThatPlayer
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
        destination_player_surface,
        tapped,
        attacking,
        face_down,
        transformed,
        converted,
        controller,
        timing,
        has_unparsed_timing_words,
        attached_to_tokens,
        excluded_subtypes,
        entry_counter_tokens,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnBackReferenceShape {
    It,
    Them,
    Demonstrative,
}

pub fn parse_return_back_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnBackReferenceShape> {
    primitives::parse_all(
        tokens,
        (
            alt((
                primitives::kw("it").value(ReturnBackReferenceShape::It),
                primitives::kw("them").value(ReturnBackReferenceShape::Them),
                primitives::any_phrase(&[
                    &["that", "card"],
                    &["that", "creature"],
                    &["that", "object"],
                    &["that", "permanent"],
                    &["those", "cards"],
                    &["those", "creatures"],
                    &["those", "objects"],
                    &["those", "permanents"],
                ])
                .value(ReturnBackReferenceShape::Demonstrative),
            )),
            primitives::sentence_end(),
        )
            .map(|(shape, ())| shape),
        "return back-reference",
    )
    .ok()
}

pub fn is_return_back_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    parse_return_back_reference_shape(tokens).is_some()
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

#[cfg(test)]
#[path = "return_shapes_inline_tests.rs"]
mod tests;

#[path = "return_shapes/zone.rs"]
mod zone_programs;
pub use zone_programs::parse_return_clause_shape;
#[path = "return_shapes/reference.rs"]
mod reference_programs;
use reference_programs::classify_target;
