use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordRepeatShape<'a> {
    Once,
    Twice,
    Count(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseDirectionShape {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseSubjectShape<'a> {
    Target(&'a [OwnedLexToken]),
    All(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumericKeywordShape {
    Bolster,
    Support,
    Adapt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestPlayerShape {
    You,
    ThatPlayerOrTargetController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordSubjectShape<'a> {
    Source(&'a [OwnedLexToken]),
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordMechanicShape<'a> {
    Amass {
        subtype: Option<Subtype>,
        amount_and_binding_tokens: &'a [OwnedLexToken],
    },
    Forage,
    Harness,
    RollD6 {
        count_tokens: &'a [OwnedLexToken],
    },
    OddEvenResult {
        odd: bool,
        action_tokens: &'a [OwnedLexToken],
    },
    Unsupported,
    Phase {
        direction: PhaseDirectionShape,
        subject: PhaseSubjectShape<'a>,
    },
    OpenAttraction,
    Behold {
        subtype: Subtype,
        count: u32,
    },
    Blight {
        amount: u32,
    },
    ManifestDread {
        repeat: KeywordRepeatShape<'a>,
    },
    ManifestTop {
        player: ManifestPlayerShape,
    },
    CloakTop {
        player: ManifestPlayerShape,
    },
    ManifestFromHand,
    Populate {
        repeat: KeywordRepeatShape<'a>,
    },
    Meld {
        result_name_tokens: &'a [OwnedLexToken],
    },
    Numeric {
        keyword: NumericKeywordShape,
        amount: u32,
    },
    Fateseal {
        count_tokens: &'a [OwnedLexToken],
    },
    DiscoverSameValue,
    Discover {
        count_tokens: &'a [OwnedLexToken],
    },
    Explore {
        subject: KeywordSubjectShape<'a>,
        repeat: KeywordRepeatShape<'a>,
    },
    Endure {
        subject: KeywordSubjectShape<'a>,
        amount_tokens: &'a [OwnedLexToken],
    },
}

fn tokens_before<'a, P>(
    input: &mut LexStream<'a>,
    minimum: usize,
    parser: P,
) -> WResult<&'a [OwnedLexToken]>
where
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    repeat_till::<_, _, (), _, _, _, _>(minimum.., any.void(), peek(parser))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn parse_subtype<'a>(input: &mut LexStream<'a>) -> WResult<Subtype> {
    primitives::word_parser_text
        .verify_map(|word| {
            parse_subtype_word(word).or_else(|| {
                crate::string_primitives::strip_suffix_char(word, 's').and_then(parse_subtype_word)
            })
        })
        .parse_next(input)
}

fn parse_creature_subtype<'a>(input: &mut LexStream<'a>) -> WResult<Subtype> {
    parse_subtype
        .verify(Subtype::is_creature_type)
        .parse_next(input)
}

fn source_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it").void(),
        primitives::kw("this").void(),
        primitives::phrase(&["this", "creature"]),
        primitives::phrase(&["this", "permanent"]),
    ))
    .parse_next(input)
}

fn classify_subject(tokens: &[OwnedLexToken]) -> KeywordSubjectShape<'_> {
    if tokens.is_empty()
        || primitives::parse_all(
            tokens,
            (source_reference, eof).void(),
            "keyword source subject",
        )
        .is_ok()
    {
        KeywordSubjectShape::Source(tokens)
    } else {
        KeywordSubjectShape::Target(tokens)
    }
}

fn parse_amass<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("amass").parse_next(input)?;
    let subtype = opt(parse_creature_subtype).parse_next(input)?;
    let amount_and_binding_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Amass {
        subtype,
        amount_and_binding_tokens,
    })
}

fn parse_forage<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    alt((primitives::kw("forage"), primitives::kw("forages"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Forage)
}

fn parse_harness<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("harness").parse_next(input)?;
    tokens_before(input, 0, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Harness)
}

fn parse_roll_d6<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("roll").parse_next(input)?;
    let count_tokens = tokens_before(input, 1, six_sided_dice)?;
    six_sided_dice.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::RollD6 { count_tokens })
}

fn six_sided_dice<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["six", "sided", "dice"]),
        (
            any.verify(|token: &&OwnedLexToken| {
                matches!(
                    token.parser_word_pieces(),
                    [six, sided] if six.text == "six" && sided.text == "sided"
                )
            }),
            primitives::kw("dice"),
        )
            .void(),
    ))
    .void()
    .parse_next(input)
}

fn parse_odd_even_result<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let odd = alt((
        primitives::kw("odd").value(true),
        primitives::kw("even").value(false),
    ))
    .parse_next(input)?;
    primitives::kw("result").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    repeat::<_, _, (), _, _>(
        0..,
        alt((primitives::kw("then"), primitives::kw("you"))).void(),
    )
    .parse_next(input)?;
    let action_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::OddEvenResult { odd, action_tokens })
}

fn parse_unsupported<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    alt((primitives::kw("dredge"), primitives::kw("warp"))).parse_next(input)?;
    tokens_before(input, 0, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Unsupported)
}

fn phase_marker<'a>(input: &mut LexStream<'a>) -> WResult<PhaseDirectionShape> {
    alt((
        (
            alt((primitives::kw("phase"), primitives::kw("phases"))),
            primitives::kw("out"),
        )
            .value(PhaseDirectionShape::Out),
        (
            alt((primitives::kw("phase"), primitives::kw("phases"))),
            primitives::kw("in"),
        )
            .value(PhaseDirectionShape::In),
    ))
    .parse_next(input)
}

fn phased_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("phased"), primitives::kw("phased-out")))
        .void()
        .parse_next(input)
}

fn parse_all_phase_subject<'a>(
    tokens: &'a [OwnedLexToken],
    direction: PhaseDirectionShape,
) -> Option<&'a [OwnedLexToken]> {
    let mut input = LexStream::new(tokens);
    let simultaneously = opt(primitives::kw("simultaneously"))
        .parse_next(&mut input)
        .ok()?
        .is_some();
    if simultaneously {
        opt(primitives::comma()).parse_next(&mut input).ok()?;
    }
    primitives::kw("all").parse_next(&mut input).ok()?;
    if direction == PhaseDirectionShape::In {
        opt(phased_word).parse_next(&mut input).ok()?;
    }
    let filter_tokens = tokens_before(&mut input, 1, eof.void()).ok()?;
    primitives::end_of_block().parse_next(&mut input).ok()?;
    Some(filter_tokens)
}

fn parse_target_phase_subject(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut input = LexStream::new(tokens);
    opt(primitives::kw("simultaneously"))
        .parse_next(&mut input)
        .ok()?;
    let target_tokens = tokens_before(&mut input, 1, eof.void()).ok()?;
    primitives::end_of_block().parse_next(&mut input).ok()?;
    Some(target_tokens)
}

fn parse_phase<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let raw_subject = tokens_before(input, 1, phase_marker.void())?;
    let direction = phase_marker.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let subject = if let Some(filter_tokens) = parse_all_phase_subject(raw_subject, direction) {
        PhaseSubjectShape::All(filter_tokens)
    } else {
        PhaseSubjectShape::Target(parse_target_phase_subject(raw_subject).ok_or_else(|| {
            primitives::backtrack_err("phase subject", "target or all-filter subject")
        })?)
    };
    Ok(KeywordMechanicShape::Phase { direction, subject })
}

fn parse_open_attraction<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    alt((
        primitives::phrase(&["open", "an", "attraction"]),
        primitives::phrase(&["opens", "an", "attraction"]),
    ))
    .parse_next(input)?;
    tokens_before(input, 0, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::OpenAttraction)
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("a"), primitives::kw("an")))
        .void()
        .parse_next(input)
}

fn parse_behold<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("behold").parse_next(input)?;
    let mut count_probe = input.clone();
    let count = if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut count_probe)
    {
        *input = count_probe;
        count
    } else {
        opt(article).parse_next(input)?;
        1
    };
    let subtype = parse_subtype.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Behold { subtype, count })
}

fn parse_blight<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("blight").parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Blight { amount })
}

fn repeat_tail<'a>(input: &mut LexStream<'a>) -> WResult<KeywordRepeatShape<'a>> {
    if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        return Ok(KeywordRepeatShape::Once);
    }
    let mut twice_probe = input.clone();
    if primitives::kw("twice").parse_next(&mut twice_probe).is_ok()
        && primitives::sentence_end()
            .parse_next(&mut twice_probe)
            .is_ok()
    {
        *input = twice_probe;
        return Ok(KeywordRepeatShape::Twice);
    }
    let count = tokens_before(
        input,
        1,
        alt((primitives::kw("time"), primitives::kw("times"))).void(),
    )?;
    alt((primitives::kw("time"), primitives::kw("times"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordRepeatShape::Count(count))
}

fn parse_manifest_dread<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::phrase(&["manifest", "dread"]).parse_next(input)?;
    let repeat = repeat_tail.parse_next(input)?;
    Ok(KeywordMechanicShape::ManifestDread { repeat })
}

fn parse_manifest_top_you<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::phrase(&["manifest", "the", "top", "card", "of", "your", "library"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::ManifestTop {
        player: ManifestPlayerShape::You,
    })
}

fn parse_cloak_top_you<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::phrase(&["cloak", "the", "top", "card", "of", "your", "library"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::CloakTop {
        player: ManifestPlayerShape::You,
    })
}

fn parse_manifest_from_hand<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::phrase(&["manifest", "a", "card", "from", "your", "hand"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::ManifestFromHand)
}

fn parse_manifest_top_that_player<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordMechanicShape<'a>> {
    alt((
        primitives::phrase(&[
            "manifest", "the", "top", "card", "of", "that", "player's", "library",
        ]),
        primitives::phrase(&[
            "manifest", "the", "top", "card", "of", "that", "players", "library",
        ]),
        primitives::phrase(&[
            "its",
            "controller",
            "manifests",
            "the",
            "top",
            "card",
            "of",
            "their",
            "library",
        ]),
        primitives::phrase(&[
            "that",
            "player",
            "manifests",
            "the",
            "top",
            "card",
            "of",
            "their",
            "library",
        ]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::ManifestTop {
        player: ManifestPlayerShape::ThatPlayerOrTargetController,
    })
}

fn parse_cloak_top_that_player<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    alt((
        primitives::phrase(&[
            "cloak", "the", "top", "card", "of", "that", "player's", "library",
        ]),
        primitives::phrase(&[
            "cloak", "the", "top", "card", "of", "that", "players", "library",
        ]),
        primitives::phrase(&[
            "its",
            "controller",
            "cloaks",
            "the",
            "top",
            "card",
            "of",
            "their",
            "library",
        ]),
        primitives::phrase(&[
            "that", "player", "cloaks", "the", "top", "card", "of", "their", "library",
        ]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::CloakTop {
        player: ManifestPlayerShape::ThatPlayerOrTargetController,
    })
}

fn parse_populate<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("populate").parse_next(input)?;
    let repeat = repeat_tail.parse_next(input)?;
    Ok(KeywordMechanicShape::Populate { repeat })
}

fn parse_meld<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("meld").parse_next(input)?;
    alt((
        primitives::kw("them").void(),
        primitives::phrase(&["those", "cards"]),
    ))
    .parse_next(input)?;
    primitives::kw("into").parse_next(input)?;
    let result_name_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Meld { result_name_tokens })
}

fn parse_numeric_keyword<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let keyword = alt((
        primitives::kw("bolster").value(NumericKeywordShape::Bolster),
        primitives::kw("support").value(NumericKeywordShape::Support),
        primitives::kw("adapt").value(NumericKeywordShape::Adapt),
    ))
    .parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Numeric { keyword, amount })
}

fn parse_fateseal<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    primitives::kw("fateseal").parse_next(input)?;
    let count_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Fateseal { count_tokens })
}

fn parse_discover<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    alt((primitives::kw("discover"), primitives::kw("discovers"))).parse_next(input)?;
    let mut same_probe = input.clone();
    if primitives::phrase(&["again", "for", "the", "same", "value"])
        .parse_next(&mut same_probe)
        .is_ok()
        && primitives::sentence_end()
            .parse_next(&mut same_probe)
            .is_ok()
    {
        *input = same_probe;
        return Ok(KeywordMechanicShape::DiscoverSameValue);
    }
    let count_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Discover { count_tokens })
}

fn parse_explore<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let subject_tokens = tokens_before(
        input,
        0,
        alt((primitives::kw("explore"), primitives::kw("explores"))).void(),
    )?;
    alt((primitives::kw("explore"), primitives::kw("explores"))).parse_next(input)?;
    let repeat = if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        KeywordRepeatShape::Once
    } else {
        let mut again_probe = input.clone();
        if primitives::kw("again").parse_next(&mut again_probe).is_ok()
            && primitives::sentence_end()
                .parse_next(&mut again_probe)
                .is_ok()
        {
            *input = again_probe;
            KeywordRepeatShape::Once
        } else {
            repeat_tail.parse_next(input)?
        }
    };
    Ok(KeywordMechanicShape::Explore {
        subject: classify_subject(subject_tokens),
        repeat,
    })
}

fn parse_endure<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let subject_tokens = tokens_before(
        input,
        0,
        alt((primitives::kw("endure"), primitives::kw("endures"))).void(),
    )?;
    alt((primitives::kw("endure"), primitives::kw("endures"))).parse_next(input)?;
    let amount_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Endure {
        subject: classify_subject(subject_tokens),
        amount_tokens,
    })
}

fn parse_keyword_mechanic_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordMechanicShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    opt(primitives::kw("you")).parse_next(input)?;
    alt((
        parse_amass,
        parse_forage,
        parse_harness,
        parse_roll_d6,
        parse_odd_even_result,
        parse_unsupported,
        parse_phase,
        parse_open_attraction,
        alt((
            parse_behold,
            parse_blight,
            parse_manifest_dread,
            parse_manifest_from_hand,
            alt((
                parse_cloak_top_you,
                parse_manifest_top_you,
                parse_cloak_top_that_player,
                parse_manifest_top_that_player,
            )),
            parse_populate,
            parse_meld,
            alt((
                parse_numeric_keyword,
                parse_fateseal,
                parse_discover,
                parse_explore,
                parse_endure,
            )),
        )),
    ))
    .parse_next(input)
}

pub(crate) fn parse_keyword_mechanic_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordMechanicShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_keyword_mechanic_lexed,
        "keyword mechanic clause",
    )
    .ok()
}

#[cfg(test)]
#[path = "keywords/tests.rs"]
mod tests;
