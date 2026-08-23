use winnow::combinator::{alt, eof, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::{EventValueSpec, Value};
use crate::grammar::{primitives, values};
use crate::lexer::{LexStream, LexedClause, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceSpanShape {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceShapeError {
    MissingThatObject,
    MissingIt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameControllerShape {
    pub cleaned_tokens: Vec<OwnedLexToken>,
    pub same_controller: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameNameFanoutVerb {
    Destroy,
    Exile,
    Return,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SameNameFanoutShape<'a> {
    Damage {
        amount: Value,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
    },
    Action {
        verb: SameNameFanoutVerb,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
        mentions_graveyard: bool,
        mentions_your_graveyard: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedColorVerb {
    Destroy,
    Exile,
    Untap,
    Get,
    Gain,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SharedColorFanoutShape<'a> {
    ExplicitGetOrGain {
        verb: SharedColorVerb,
        duration_tokens: Option<&'a [OwnedLexToken]>,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
        action_tokens: &'a [OwnedLexToken],
    },
    Action {
        verb: SharedColorVerb,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
    },
    Damage {
        amount: Value,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
    },
    Prevent {
        amount: Value,
        first_target_tokens: &'a [OwnedLexToken],
        filter_tokens: &'a [OwnedLexToken],
    },
    SubjectGetOrGain {
        verb: SharedColorVerb,
        subject_tokens: &'a [OwnedLexToken],
        split_targets: Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])>,
        action_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerSurface {
    TargetPlayerOrControllerOfTarget,
    ContextualTargetPlayer,
    Opponent,
    You,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamagePartShape {
    EachPlayer {
        opponent_only: bool,
    },
    EachObject {
        filter_tokens: Vec<OwnedLexToken>,
        controller: Option<ControllerSurface>,
    },
    TargetYou(Vec<OwnedLexToken>),
    TargetOpponent(Vec<OwnedLexToken>),
    TargetTokens {
        tokens: Vec<OwnedLexToken>,
        controller: Option<ControllerSurface>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundDamageShape {
    pub source_tokens: Vec<OwnedLexToken>,
    pub amount: Value,
    pub left_tokens: Vec<OwnedLexToken>,
    pub right_tokens: Vec<OwnedLexToken>,
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn any_semantic_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| token.as_word().is_some())
        .void()
        .parse_next(input)
}

fn same_name_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("with").parse_next(input)?;
    winnow::combinator::opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["same", "name", "as", "that"]).parse_next(input)?;
    any_semantic_word.parse_next(input)
}

fn incomplete_same_name_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("with").parse_next(input)?;
    winnow::combinator::opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["same", "name", "as"]).parse_next(input)
}

fn find_span_with<'a, P, F>(
    tokens: &'a [OwnedLexToken],
    make_parser: F,
) -> Option<ReferenceSpanShape>
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let (start, (), rest) = primitives::find_prefix(tokens, make_parser)?;
    Some(ReferenceSpanShape {
        start,
        end: tokens.len().saturating_sub(rest.len()),
    })
}

pub fn parse_same_name_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<ReferenceSpanShape>, ReferenceShapeError> {
    if let Some(shape) = find_span_with(tokens, || same_name_reference) {
        return Ok(Some(shape));
    }
    if find_span_with(tokens, || incomplete_same_name_reference).is_some() {
        return Err(ReferenceShapeError::MissingThatObject);
    }
    Ok(None)
}

fn same_controller_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            primitives::phrase(&["that", "player"]),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .void(),
        (
            primitives::phrase(&["its", "controller"]),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .void(),
        (
            primitives::kw("that"),
            alt((
                primitives::kw("creature"),
                primitives::kw("permanent"),
                primitives::kw("card"),
            )),
            primitives::kw("controller"),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .void(),
    ))
    .parse_next(input)
}

pub fn strip_same_controller_shape(tokens: &[OwnedLexToken]) -> SameControllerShape {
    let mut cleaned_tokens = Vec::with_capacity(tokens.len());
    let mut remaining = tokens;
    let mut same_controller = false;
    while let Some((start, (), rest)) =
        primitives::find_prefix(remaining, || same_controller_reference)
    {
        cleaned_tokens.extend_from_slice(&remaining[..start]);
        remaining = rest;
        same_controller = true;
    }
    cleaned_tokens.extend_from_slice(remaining);
    SameControllerShape {
        cleaned_tokens,
        same_controller,
    }
}

fn shares_color_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("that").parse_next(input)?;
    alt((primitives::kw("shares"), primitives::kw("share"))).parse_next(input)?;
    primitives::phrase(&["a", "color", "with", "it"])
        .void()
        .parse_next(input)
}

fn incomplete_shares_color_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("that").parse_next(input)?;
    alt((primitives::kw("shares"), primitives::kw("share"))).parse_next(input)?;
    primitives::phrase(&["a", "color", "with"])
        .void()
        .parse_next(input)
}

pub fn parse_shares_color_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<ReferenceSpanShape>, ReferenceShapeError> {
    if let Some(shape) = find_span_with(tokens, || shares_color_reference) {
        return Ok(Some(shape));
    }
    if find_span_with(tokens, || incomplete_shares_color_reference).is_some() {
        return Err(ReferenceShapeError::MissingIt);
    }
    Ok(None)
}

fn split_on_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let (start, (), rest) = primitives::find_prefix(tokens, || primitives::phrase(phrase).void())?;
    Some((&tokens[..start], rest))
}

fn contains_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word)).is_some()
}

fn exact_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        (
            semantic_phrase(phrase),
            repeat::<_, _, (), _, _>(0.., semantic_noise),
            eof,
        )
            .void(),
        "fanout phrase",
    )
    .is_ok()
}

fn fanout_verb<'a>(input: &mut LexStream<'a>) -> WResult<SharedColorVerb> {
    alt((
        primitives::kw("destroy").value(SharedColorVerb::Destroy),
        primitives::kw("destroys").value(SharedColorVerb::Destroy),
        primitives::kw("exile").value(SharedColorVerb::Exile),
        primitives::kw("exiles").value(SharedColorVerb::Exile),
        primitives::kw("untap").value(SharedColorVerb::Untap),
        primitives::kw("untaps").value(SharedColorVerb::Untap),
        alt((
            primitives::kw("get").value(SharedColorVerb::Get),
            primitives::kw("gets").value(SharedColorVerb::Get),
            primitives::kw("gain").value(SharedColorVerb::Gain),
            primitives::kw("gains").value(SharedColorVerb::Gain),
        )),
    ))
    .parse_next(input)
}

fn deal_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)
}

fn parse_damage_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["that", "much"]).void())
    {
        return Some((
            Value::EventValue(EventValueSpec::Amount),
            tokens.len().saturating_sub(rest.len()),
        ));
    }
    values::parse_value_prefix_lexed(tokens)
}

fn last_word_index(tokens: &[OwnedLexToken], word: &'static str) -> Option<usize> {
    let mut base = 0usize;
    let mut remaining = tokens;
    let mut last = None;
    while let Some((idx, _, rest)) = primitives::find_prefix(remaining, || primitives::kw(word)) {
        last = Some(base + idx);
        let consumed = remaining.len().saturating_sub(rest.len());
        base += consumed;
        remaining = rest;
    }
    last
}

pub fn parse_same_name_fanout_shape(tokens: &[OwnedLexToken]) -> Option<SameNameFanoutShape<'_>> {
    let tokens = trimmed(tokens);
    if let Some((deal_start, (), deal_rest)) = primitives::find_prefix(tokens, || deal_verb) {
        let source_tokens = trimmed(&tokens[..deal_start]);
        let deal_is_valid = deal_start == 0 || {
            let words = crate::lexer::TokenWordView::new(source_tokens).to_word_refs();
            crate::util::is_source_reference_words(&words)
        };
        if deal_is_valid
            && let Some((amount, used)) = parse_damage_amount(deal_rest)
            && let Some((_, after_damage)) = primitives::parse_prefix(
                &deal_rest[used..],
                (
                    primitives::kw("damage"),
                    winnow::combinator::opt(primitives::kw("to")),
                ),
            )
            && let Some((first_target_tokens, filter_tokens)) =
                split_on_phrase(after_damage, &["and", "each", "other"])
        {
            let first_target_tokens = trimmed(first_target_tokens);
            let filter_tokens = trimmed(filter_tokens);
            if !first_target_tokens.is_empty()
                && !filter_tokens.is_empty()
                && contains_word(first_target_tokens, "target")
            {
                return Some(SameNameFanoutShape::Damage {
                    amount,
                    first_target_tokens,
                    filter_tokens,
                });
            }
        }
    }

    let (verb, after_verb) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("destroy").value(SameNameFanoutVerb::Destroy),
            primitives::kw("exile").value(SameNameFanoutVerb::Exile),
            primitives::kw("return").value(SameNameFanoutVerb::Return),
        )),
    )?;
    let (first_target_tokens, after_other) = split_on_phrase(after_verb, &["and", "all", "other"])?;
    let first_target_tokens = trimmed(first_target_tokens);
    if first_target_tokens.is_empty() || !contains_word(first_target_tokens, "target") {
        return None;
    }
    let filter_tokens = if verb == SameNameFanoutVerb::Return {
        let to_idx = last_word_index(after_other, "to")?;
        let destination = trimmed(&after_other[to_idx + 1..]);
        if !contains_word(destination, "hand") && !contains_word(destination, "hands") {
            return None;
        }
        trimmed(&after_other[..to_idx])
    } else {
        trimmed(after_other)
    };
    (!filter_tokens.is_empty()).then_some(SameNameFanoutShape::Action {
        verb,
        first_target_tokens,
        filter_tokens,
        mentions_graveyard: contains_word(tokens, "graveyard"),
        mentions_your_graveyard: primitives::find_prefix(tokens, || {
            primitives::phrase(&["your", "graveyard"]).void()
        })
        .is_some(),
    })
}

pub fn split_leading_end_of_turn_shape(
    tokens: &[OwnedLexToken],
) -> (Option<&[OwnedLexToken]>, &[OwnedLexToken]) {
    let Some(((), body)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["until", "end", "of", "turn"]).void(),
    ) else {
        return (None, tokens);
    };
    let body = trimmed(body);
    if body.is_empty() {
        return (None, tokens);
    }
    let duration_len = tokens.len().saturating_sub(body.len());
    (Some(&tokens[..duration_len]), body)
}

pub fn strip_radiance_label(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        (
            primitives::kw("radiance"),
            repeat::<_, _, (), _, _>(0.., semantic_noise),
        )
            .void(),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens)
}

fn explicit_get_or_gain_shape(tokens: &[OwnedLexToken]) -> Option<SharedColorFanoutShape<'_>> {
    let (duration_tokens, body_tokens) = split_leading_end_of_turn_shape(tokens);
    let (first_target_tokens, after_other) =
        split_on_phrase(body_tokens, &["and", "each", "other"])?;
    let (verb_idx, verb, _tail_tokens) = primitives::find_prefix(after_other, || fanout_verb)?;
    if !matches!(verb, SharedColorVerb::Get | SharedColorVerb::Gain) {
        return None;
    }
    let first_target_tokens = trimmed(first_target_tokens);
    let filter_tokens = trimmed(&after_other[..verb_idx]);
    let action_tokens = trimmed(&after_other[verb_idx..]);
    if first_target_tokens.is_empty()
        || filter_tokens.is_empty()
        || action_tokens.is_empty()
        || !contains_word(first_target_tokens, "target")
    {
        return None;
    }
    Some(SharedColorFanoutShape::ExplicitGetOrGain {
        verb,
        duration_tokens,
        first_target_tokens,
        filter_tokens,
        action_tokens,
    })
}

pub fn parse_shared_color_fanout_shape(
    tokens: &[OwnedLexToken],
) -> Option<SharedColorFanoutShape<'_>> {
    let tokens = strip_radiance_label(trimmed(tokens));
    if let Some(shape) = explicit_get_or_gain_shape(tokens) {
        return Some(shape);
    }

    if let Some((verb_idx, verb, after_verb)) = primitives::find_prefix(tokens, || fanout_verb) {
        if matches!(
            verb,
            SharedColorVerb::Destroy | SharedColorVerb::Exile | SharedColorVerb::Untap
        ) {
            let (first_target_tokens, filter_tokens) =
                split_on_phrase(after_verb, &["and", "each", "other"])?;
            let first_target_tokens = trimmed(first_target_tokens);
            let filter_tokens = trimmed(filter_tokens);
            if first_target_tokens.is_empty()
                || filter_tokens.is_empty()
                || !contains_word(first_target_tokens, "target")
            {
                return None;
            }
            return Some(SharedColorFanoutShape::Action {
                verb,
                first_target_tokens,
                filter_tokens,
            });
        }

        if matches!(verb, SharedColorVerb::Get | SharedColorVerb::Gain) && verb_idx > 0 {
            let subject_tokens = trimmed(&tokens[..verb_idx]);
            let split_targets = split_on_phrase(subject_tokens, &["and", "each", "other"])
                .map(|(first, second)| (trimmed(first), trimmed(second)))
                .filter(|(first, second)| {
                    !first.is_empty() && !second.is_empty() && contains_word(first, "target")
                });
            return Some(SharedColorFanoutShape::SubjectGetOrGain {
                verb,
                subject_tokens,
                split_targets,
                action_tokens: trimmed(&tokens[verb_idx..]),
            });
        }
    }

    if let Some((_deal_idx, (), after_deal)) = primitives::find_prefix(tokens, || deal_verb) {
        let (amount, used) = parse_damage_amount(after_deal)?;
        let (_, after_damage) = primitives::parse_prefix(
            &after_deal[used..],
            (
                primitives::kw("damage"),
                winnow::combinator::opt(primitives::kw("to")),
            ),
        )?;
        let (first_target_tokens, filter_tokens) =
            split_on_phrase(after_damage, &["and", "each", "other"])?;
        let first_target_tokens = trimmed(first_target_tokens);
        let filter_tokens = trimmed(filter_tokens);
        if !first_target_tokens.is_empty()
            && !filter_tokens.is_empty()
            && contains_word(first_target_tokens, "target")
        {
            return Some(SharedColorFanoutShape::Damage {
                amount,
                first_target_tokens,
                filter_tokens,
            });
        }
    }

    let (_, after_prevent) = primitives::parse_prefix(tokens, primitives::kw("prevent"))?;
    let (_, after_next) = primitives::parse_prefix(
        after_prevent,
        (
            winnow::combinator::opt(primitives::kw("the")),
            primitives::kw("next"),
        ),
    )?;
    let (amount, used) = values::parse_value_prefix_lexed(after_next)?;
    let (_, scope_tokens) = primitives::parse_prefix(
        &after_next[used..],
        primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]),
    )?;
    let (scope_tokens, ()) = primitives::split_lexed_once_before_suffix(scope_tokens, 1, || {
        (primitives::phrase(&["this", "turn"]), eof).void()
    })?;
    let (first_target_tokens, filter_tokens) =
        split_on_phrase(scope_tokens, &["and", "each", "other"])?;
    let first_target_tokens = trimmed(first_target_tokens);
    let filter_tokens = trimmed(filter_tokens);
    (!first_target_tokens.is_empty()
        && !filter_tokens.is_empty()
        && contains_word(first_target_tokens, "target"))
    .then_some(SharedColorFanoutShape::Prevent {
        amount,
        first_target_tokens,
        filter_tokens,
    })
}

fn strip_trailing_damage_noise(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trimmed(tokens);
    if let Some((head, ())) = primitives::split_lexed_once_before_suffix(tokens, 0, || {
        (
            primitives::kw("instead"),
            repeat::<_, _, (), _, _>(0.., semantic_noise),
            eof,
        )
            .void()
    }) {
        return trimmed(head).to_vec();
    }
    tokens.to_vec()
}

fn strip_trailing_where(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    primitives::find_prefix(tokens, || primitives::kw("where"))
        .map(|(idx, _, _)| strip_trailing_damage_noise(&tokens[..idx]))
        .unwrap_or_else(|| strip_trailing_damage_noise(tokens))
}

fn strip_semantic_suffix<'a>(
    tokens: &'a [OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> Option<&'a [OwnedLexToken]> {
    for phrase in phrases {
        if let Some((head, ())) = primitives::split_lexed_once_before_suffix(tokens, 0, || {
            (
                semantic_phrase(phrase),
                repeat::<_, _, (), _, _>(0.., semantic_noise),
                eof,
            )
                .void()
        }) {
            return Some(trimmed(head));
        }
    }
    None
}

fn strip_controller_tail(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, Option<ControllerSurface>) {
    const TARGET_OR_CONTROLLER: &[&[&str]] = &[
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "controls",
        ],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "controller",
            "controls",
        ],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "control",
        ],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "controller",
            "control",
        ],
    ];
    const CONTEXTUAL: &[&[&str]] = &[
        &["that", "player", "controls"],
        &["that", "player", "control"],
        &["they", "control"],
        &["they", "controls"],
    ];
    const OPPONENT: &[&[&str]] = &[
        &["your", "opponents", "control"],
        &["your", "opponent", "controls"],
    ];
    const YOU: &[&[&str]] = &[
        &["you", "control"],
        &["you", "controls"],
        &["your", "control"],
    ];
    for (phrases, surface) in [
        (
            TARGET_OR_CONTROLLER,
            ControllerSurface::TargetPlayerOrControllerOfTarget,
        ),
        (CONTEXTUAL, ControllerSurface::ContextualTargetPlayer),
        (OPPONENT, ControllerSurface::Opponent),
        (YOU, ControllerSurface::You),
    ] {
        if let Some(base) = strip_semantic_suffix(tokens, phrases) {
            return (strip_trailing_damage_noise(base), Some(surface));
        }
    }
    (tokens.to_vec(), None)
}

fn exact_any(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases.iter().any(|phrase| exact_phrase(tokens, phrase))
}

fn parse_each_damage_part_shape(tokens: &[OwnedLexToken]) -> Option<DamagePartShape> {
    const PLAYER_OR_OPPONENT: &[&[&str]] =
        &[&["player"], &["players"], &["opponent"], &["opponents"]];
    const OPPONENT: &[&[&str]] = &[&["opponent"], &["opponents"]];
    let tokens = strip_trailing_damage_noise(tokens);
    if exact_any(&tokens, PLAYER_OR_OPPONENT) {
        return Some(DamagePartShape::EachPlayer {
            opponent_only: exact_any(&tokens, OPPONENT),
        });
    }
    if primitives::parse_prefix(
        &tokens,
        alt((primitives::kw("player"), primitives::kw("players"))),
    )
    .is_some()
    {
        return None;
    }
    let (filter_tokens, controller) = strip_controller_tail(&tokens);
    (!filter_tokens.is_empty()).then_some(DamagePartShape::EachObject {
        filter_tokens,
        controller,
    })
}

pub fn parse_damage_part_shape(
    tokens: &[OwnedLexToken],
    require_each: bool,
) -> Option<DamagePartShape> {
    let tokens = strip_trailing_damage_noise(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(
        &tokens,
        alt((primitives::kw("each"), primitives::kw("all"))).void(),
    ) {
        return parse_each_damage_part_shape(rest);
    }
    if require_each {
        return parse_each_damage_part_shape(&tokens);
    }
    if exact_phrase(&tokens, &["you"]) {
        return Some(DamagePartShape::TargetYou(tokens));
    }
    if exact_any(&tokens, &[&["opponent"], &["opponents"]]) {
        return Some(DamagePartShape::TargetOpponent(tokens));
    }
    if !contains_word(&tokens, "target") {
        return None;
    }
    let (target_tokens, controller) = strip_controller_tail(&tokens);
    (!target_tokens.is_empty()).then_some(DamagePartShape::TargetTokens {
        tokens: target_tokens,
        controller,
    })
}

fn equal_damage_amount_and_targets(tokens: &[OwnedLexToken]) -> Option<(Value, &[OwnedLexToken])> {
    let (_, after_equal) =
        primitives::parse_prefix(tokens, primitives::phrase(&["damage", "equal", "to"]))?;
    let mut offset = 0usize;
    let mut remaining = after_equal;
    while let Some((idx, _, rest)) = primitives::find_prefix(remaining, || primitives::kw("to")) {
        let split = offset + idx;
        let target_tail = trimmed(rest);
        if primitives::parse_prefix(
            target_tail,
            alt((
                primitives::kw("each"),
                primitives::kw("all"),
                primitives::kw("target"),
                primitives::kw("you"),
                primitives::kw("opponent"),
                primitives::kw("opponents"),
                primitives::kw("player"),
                primitives::kw("players"),
            )),
        )
        .is_some()
        {
            let amount_tokens = trimmed(&after_equal[..split]);
            let (amount, used) = values::parse_value_prefix_lexed(amount_tokens)?;
            if used == amount_tokens.len() {
                return Some((amount, target_tail));
            }
        }
        let consumed = remaining.len().saturating_sub(rest.len());
        offset += consumed;
        remaining = rest;
    }
    None
}

pub fn parse_compound_damage_shape(tokens: &[OwnedLexToken]) -> Option<CompoundDamageShape> {
    let tokens = trimmed(tokens);
    let (deal_idx, (), after_deal) = primitives::find_prefix(tokens, || deal_verb)?;
    let source_tokens = trimmed(&tokens[..deal_idx]).to_vec();
    let (amount, target_tokens) =
        if let Some((amount, targets)) = equal_damage_amount_and_targets(after_deal) {
            (amount, targets.to_vec())
        } else {
            let (amount, used) = parse_damage_amount(after_deal)?;
            let (_, targets) = primitives::parse_prefix(
                &after_deal[used..],
                (
                    primitives::kw("damage"),
                    winnow::combinator::opt(primitives::kw("to")),
                ),
            )?;
            (amount, targets.to_vec())
        };
    let target_tokens = strip_trailing_where(&target_tokens);
    let (left_tokens, right_tokens) = split_on_phrase(&target_tokens, &["and", "each"])
        .or_else(|| split_on_phrase(&target_tokens, &["and", "all"]))?;
    let left_tokens = trimmed(left_tokens).to_vec();
    let right_tokens = trimmed(right_tokens).to_vec();
    (!left_tokens.is_empty() && !right_tokens.is_empty()).then_some(CompoundDamageShape {
        source_tokens,
        amount,
        left_tokens,
        right_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_reference_and_controller_shapes() {
        let same_name = tokens("creatures with the same name as that creature");
        assert!(
            parse_same_name_reference_span(&same_name)
                .unwrap()
                .is_some()
        );
        let controller = tokens("creatures that player controls");
        let shape = strip_same_controller_shape(&controller);
        assert!(shape.same_controller);
        assert_eq!(shape.cleaned_tokens.len(), 1);

        let destroy = tokens(
            "Destroy target artifact and all other artifacts with the same name as that artifact.",
        );
        assert!(matches!(
            parse_same_name_fanout_shape(&destroy),
            Some(SameNameFanoutShape::Action {
                verb: SameNameFanoutVerb::Destroy,
                ..
            })
        ));
    }

    #[test]
    fn parses_compound_damage_surface() {
        let damage = tokens("deal 2 damage to target creature and each opponent");
        let shape = parse_compound_damage_shape(&damage).unwrap();
        assert_eq!(shape.amount, Value::Fixed(2));
        assert!(matches!(
            parse_damage_part_shape(&shape.right_tokens, true),
            Some(DamagePartShape::EachPlayer {
                opponent_only: true
            })
        ));
    }
}
