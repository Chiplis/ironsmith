use super::super::*;

use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreventNextDamageShape<'a> {
    pub(crate) amount_tokens: &'a [OwnedLexToken],
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) source_of_your_choice: bool,
    pub(crate) protects_you_and_permanents_you_control: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct PreventNextTimeDamageShape<'a> {
    pub(crate) source: DamageSourceShape<'a>,
    pub(crate) target: DamageTargetShape<'a>,
    pub(crate) reflect_damage_to_source_controller: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamageTargetShape<'a> {
    AnyTarget,
    You,
    Target(&'a [OwnedLexToken]),
}
#[derive(Debug, Clone)]
pub(crate) enum DamageSourceShape<'a> {
    Choice,
    ChoiceMatching(ObjectFilter),
    Target(&'a [OwnedLexToken]),
    Tagged {
        card_type: Option<CardType>,
        source_tokens: &'a [OwnedLexToken],
    },
    Filter(ObjectFilter),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedirectDamageDestinationShape<'a> {
    SourceObject,
    Controller,
    Target(&'a [OwnedLexToken]),
    TargetOfChoice(&'a [OwnedLexToken]),
}
#[derive(Debug, Clone)]
pub(crate) enum RedirectNextDamageShape<'a> {
    AllToYouAndPermanents {
        other: bool,
        destination_tokens: &'a [OwnedLexToken],
    },
    AllBySourceToSourceController {
        source_tokens: &'a [OwnedLexToken],
    },
    AllToTargetByChosenSource {
        target_tokens: &'a [OwnedLexToken],
        destination: RedirectDamageDestinationShape<'a>,
    },
    NextTime {
        source: DamageSourceShape<'a>,
        target_tokens: &'a [OwnedLexToken],
        destination: RedirectDamageDestinationShape<'a>,
    },
    NextAmount {
        amount_tokens: &'a [OwnedLexToken],
        protected_tokens: Option<&'a [OwnedLexToken]>,
        destination: RedirectDamageDestinationShape<'a>,
    },
}
fn tokens_before<'a, P>(input: &mut LexStream<'a>, parser: P) -> WResult<&'a [OwnedLexToken]>
where
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(parser))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn one_or_more_tokens_before<'a, P>(
    input: &mut LexStream<'a>,
    parser: P,
) -> WResult<&'a [OwnedLexToken]>
where
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parser))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn source_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::phrase(&["this", "permanent"]),
        primitives::kw("this").void(),
        primitives::kw("it").void(),
    ))
    .parse_next(input)
}

fn you_and_permanents_filter<'a>(input: &mut LexStream<'a>) -> WResult<(bool, ObjectFilter)> {
    primitives::kw("you").parse_next(input)?;
    alt((
        primitives::kw("and/or").void(),
        primitives::phrase(&["and", "or"]),
        primitives::kw("and").void(),
    ))
    .parse_next(input)?;
    let other = opt(primitives::kw("other")).parse_next(input)?.is_some();
    let creatures = alt((
        alt((primitives::kw("creature"), primitives::kw("creatures"))).value(true),
        alt((primitives::kw("permanent"), primitives::kw("permanents"))).value(false),
    ))
    .parse_next(input)?;
    primitives::phrase(&["you", "control"]).parse_next(input)?;
    let filter = if creatures {
        ObjectFilter::creature().you_control()
    } else {
        ObjectFilter::permanent().you_control()
    };
    Ok((other, if other { filter.other() } else { filter }))
}

fn you_and_permanents<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    you_and_permanents_filter
        .map(|(other, _)| other)
        .parse_next(input)
}

pub(crate) fn parse_you_and_permanents_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    primitives::parse_all(
        tokens,
        (you_and_permanents_filter, winnow::combinator::eof).map(|((_, filter), _)| filter),
        "you and matching permanents",
    )
    .ok()
}

fn source_of_your_choice<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(alt((primitives::kw("a"), primitives::kw("the")))).parse_next(input)?;
    primitives::phrase(&["source", "of", "your", "choice"]).parse_next(input)
}

fn has_source_of_your_choice(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), source_of_your_choice)
        .parse_next(&mut input)
        .is_ok()
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .void()
    .parse_next(input)
}

fn filter_descriptor_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    opt(article).parse_next(input)?;
    let descriptor = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek((opt(primitives::kw("source")), winnow::combinator::eof).void()),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::kw("source")).parse_next(input)?;
    winnow::combinator::eof.parse_next(input)?;
    Ok(descriptor)
}

fn is_filter_connector(token: &OwnedLexToken) -> bool {
    primitives::parse_all(
        std::slice::from_ref(token),
        alt((primitives::kw("and"), primitives::kw("or"))).void(),
        "damage source connector",
    )
    .is_ok()
}

fn is_shadow_word(token: &OwnedLexToken) -> bool {
    primitives::parse_all(
        std::slice::from_ref(token),
        primitives::kw("shadow").void(),
        "damage source shadow",
    )
    .is_ok()
}

fn damage_source_filter_from_descriptor(descriptor: &[OwnedLexToken]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    let mut colors: Option<crate::color::ColorSet> = None;
    let mut saw_chosen = false;
    for token in descriptor {
        if is_filter_connector(token) {
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if word.eq_ignore_ascii_case("chosen") {
            saw_chosen = true;
            continue;
        }
        if saw_chosen && word.eq_ignore_ascii_case("type") {
            filter.chosen_creature_type = true;
            saw_chosen = false;
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(
                colors
                    .unwrap_or_else(crate::color::ColorSet::new)
                    .union(color),
            );
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if filter
                .card_types
                .iter()
                .all(|existing| existing != &card_type)
            {
                filter.card_types.push(card_type);
            }
            continue;
        }
        if is_shadow_word(token) {
            filter = filter.with_static_ability(StaticAbilityId::Shadow);
        }
    }
    filter.colors = colors;
    filter
}

fn damage_source_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let descriptor = primitives::parse_all(
        tokens,
        filter_descriptor_tokens,
        "damage source filter descriptor",
    )
    .ok()?;
    Some(damage_source_filter_from_descriptor(descriptor))
}

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case(expected))
}

/// Preserve both the fact that a source is chosen and the restrictions on
/// that choice. Oracle text uses both "artifact source of your choice" and
/// "creature of your choice with shadow" orderings.
fn damage_source_choice_filter(tokens: &[OwnedLexToken]) -> Option<Option<ObjectFilter>> {
    let choice_index = tokens.windows(3).position(|window| {
        token_is_word(&window[0], "of")
            && token_is_word(&window[1], "your")
            && token_is_word(&window[2], "choice")
    })?;

    let mut descriptor = tokens[..choice_index].to_vec();
    if descriptor
        .last()
        .is_some_and(|token| token_is_word(token, "source"))
    {
        descriptor.pop();
    }
    descriptor.extend_from_slice(&tokens[choice_index + 3..]);
    while descriptor.first().is_some_and(|token| {
        token_is_word(token, "a") || token_is_word(token, "an") || token_is_word(token, "the")
    }) {
        descriptor.remove(0);
    }

    let filter = damage_source_filter_from_descriptor(&descriptor);
    if filter == ObjectFilter::default() {
        Some(None)
    } else {
        Some(Some(filter))
    }
}

fn tagged_source_kind<'a>(input: &mut LexStream<'a>) -> WResult<Option<CardType>> {
    primitives::kw("that").parse_next(input)?;
    let kind = opt(any).parse_next(input)?;
    winnow::combinator::eof.parse_next(input)?;
    Ok(kind
        .and_then(OwnedLexToken::as_word)
        .and_then(parse_card_type))
}

fn classify_damage_source(tokens: &[OwnedLexToken]) -> Option<DamageSourceShape<'_>> {
    if let Some(filter) = damage_source_choice_filter(tokens) {
        return Some(match filter {
            Some(filter) => DamageSourceShape::ChoiceMatching(filter),
            None => DamageSourceShape::Choice,
        });
    }
    if primitives::parse_prefix(tokens, primitives::kw("target")).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["another", "target"])).is_some()
    {
        return Some(DamageSourceShape::Target(tokens));
    }
    if primitives::parse_all(
        tokens,
        (primitives::kw("it"), winnow::combinator::eof).void(),
        "tagged damage source",
    )
    .is_ok()
    {
        return Some(DamageSourceShape::Tagged {
            card_type: None,
            source_tokens: tokens,
        });
    }
    if let Ok(card_type) = primitives::parse_all(tokens, tagged_source_kind, "tagged source kind") {
        return Some(DamageSourceShape::Tagged {
            card_type,
            source_tokens: tokens,
        });
    }
    damage_source_filter(tokens).map(DamageSourceShape::Filter)
}

fn classify_damage_target(tokens: &[OwnedLexToken]) -> DamageTargetShape<'_> {
    if tokens.is_empty() {
        DamageTargetShape::AnyTarget
    } else if primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["any", "target"]),
            winnow::combinator::eof,
        )
            .void(),
        "any damage target",
    )
    .is_ok()
    {
        DamageTargetShape::Target(tokens)
    } else if primitives::parse_all(
        tokens,
        (primitives::kw("you"), winnow::combinator::eof).void(),
        "you damage target",
    )
    .is_ok()
    {
        DamageTargetShape::You
    } else {
        DamageTargetShape::Target(tokens)
    }
}

fn parse_prevent_next_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventNextDamageShape<'a>> {
    primitives::kw("prevent").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("next").parse_next(input)?;
    let amount_tokens = any.void().take().parse_next(input)?;
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]).parse_next(input)?;
    let target_tokens = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    let source_of_your_choice = opt((primitives::kw("by"), source_of_your_choice))
        .parse_next(input)?
        .is_some();
    primitives::sentence_end().parse_next(input)?;
    let protects_you_and_permanents_you_control = primitives::parse_all(
        target_tokens,
        (you_and_permanents, winnow::combinator::eof).map(|(_, _)| ()),
        "prevent-next combined target",
    )
    .is_ok();
    Ok(PreventNextDamageShape {
        amount_tokens,
        target_tokens,
        source_of_your_choice,
        protects_you_and_permanents_you_control,
    })
}

pub(crate) fn parse_prevent_next_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreventNextDamageShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_prevent_next_damage_lexed,
        "prevent next damage",
    )
    .ok()
}

fn reflect_tail<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    primitives::phrase(&[
        "prevent",
        "that",
        "damage",
        "if",
        "damage",
        "is",
        "prevented",
        "this",
        "way",
    ])
    .parse_next(input)?;
    tokens_before(input, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(true)
}

fn simple_prevent_tail<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    primitives::phrase(&["prevent", "that", "damage"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(false)
}

fn parse_prevent_next_time_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventNextTimeDamageShape<'a>> {
    primitives::phrase(&["the", "next", "time"]).parse_next(input)?;
    let source_tokens = one_or_more_tokens_before(input, primitives::kw("would").void())?;
    primitives::phrase(&["would", "deal", "damage"]).parse_next(input)?;
    opt(primitives::kw("to")).parse_next(input)?;
    let target_tokens = tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let reflect_damage_to_source_controller =
        alt((reflect_tail, simple_prevent_tail)).parse_next(input)?;
    Ok(PreventNextTimeDamageShape {
        source: classify_damage_source(source_tokens)
            .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?,
        target: classify_damage_target(target_tokens),
        reflect_damage_to_source_controller,
    })
}

pub(crate) fn parse_prevent_next_time_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreventNextTimeDamageShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_prevent_next_time_damage_lexed,
        "prevent next time damage",
    )
    .ok()
}

fn parse_all_to_you_and_permanents<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&[
        "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ])
    .parse_next(input)?;
    let other = you_and_permanents.parse_next(input)?;
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::AllToYouAndPermanents {
        other,
        destination_tokens,
    })
}

fn source_controller_destination<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["that", "spell's", "controller"]),
        primitives::phrase(&["that", "spells", "controller"]),
        primitives::phrase(&["that", "source's", "controller"]),
        primitives::phrase(&["that", "sources", "controller"]),
    ))
    .parse_next(input)
}

fn parse_all_by_source<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&[
        "all", "damage", "that", "would", "be", "dealt", "this", "turn", "by",
    ])
    .parse_next(input)?;
    let source_tokens =
        one_or_more_tokens_before(input, primitives::phrase(&["is", "dealt", "to"]))?;
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    source_controller_destination.parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::AllBySourceToSourceController { source_tokens })
}

fn destination_to_source_or_controller<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectDamageDestinationShape<'a>> {
    alt((
        source_reference.value(RedirectDamageDestinationShape::SourceObject),
        primitives::kw("you").value(RedirectDamageDestinationShape::Controller),
    ))
    .parse_next(input)
}

fn parse_all_to_target_by_choice<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["all", "damage", "that", "would", "be", "dealt", "to"])
        .parse_next(input)?;
    let target_tokens = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn", "by"]).parse_next(input)?;
    let source_tokens =
        one_or_more_tokens_before(input, primitives::phrase(&["is", "dealt", "to"]))?;
    if !has_source_of_your_choice(source_tokens) {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    }
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    let destination = destination_to_source_or_controller.parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::AllToTargetByChosenSource {
        target_tokens,
        destination,
    })
}

fn classify_next_time_destination(
    tokens: &[OwnedLexToken],
) -> Option<RedirectDamageDestinationShape<'_>> {
    if primitives::parse_all(
        tokens,
        (source_reference, winnow::combinator::eof).void(),
        "redirect source destination",
    )
    .is_ok()
    {
        return Some(RedirectDamageDestinationShape::SourceObject);
    }
    if primitives::parse_all(
        tokens,
        (primitives::kw("you"), winnow::combinator::eof).void(),
        "redirect controller destination",
    )
    .is_ok()
    {
        return Some(RedirectDamageDestinationShape::Controller);
    }
    let is_target = primitives::parse_prefix(tokens, primitives::kw("target")).is_some();
    let mentions_choice = {
        let mut input = LexStream::new(tokens);
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("choice"))
            .parse_next(&mut input)
            .is_ok()
    };
    if is_target && mentions_choice {
        Some(RedirectDamageDestinationShape::TargetOfChoice(tokens))
    } else {
        is_target.then_some(RedirectDamageDestinationShape::Target(tokens))
    }
}

fn next_time_tail<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    alt((
        primitives::phrase(&["that", "damage", "is", "dealt", "to"]),
        primitives::phrase(&["that", "source", "deals", "that", "damage", "to"]),
    ))
    .parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(destination_tokens)
}

fn parse_next_time<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next", "time"]).parse_next(input)?;
    let source_tokens = one_or_more_tokens_before(input, primitives::kw("would").void())?;
    primitives::phrase(&["would", "deal", "damage", "to"]).parse_next(input)?;
    let target_tokens = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let destination_tokens = next_time_tail.parse_next(input)?;
    let destination = classify_next_time_destination(destination_tokens)
        .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?;
    Ok(RedirectNextDamageShape::NextTime {
        source: classify_damage_source(source_tokens)
            .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?,
        target_tokens,
        destination,
    })
}

fn classify_next_amount_destination(
    tokens: &[OwnedLexToken],
) -> RedirectDamageDestinationShape<'_> {
    if primitives::parse_all(
        tokens,
        (primitives::kw("you"), winnow::combinator::eof).void(),
        "redirect amount controller destination",
    )
    .is_ok()
    {
        RedirectDamageDestinationShape::Controller
    } else {
        RedirectDamageDestinationShape::Target(tokens)
    }
}

fn parse_next_amount<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let amount_tokens = any.void().take().parse_next(input)?;
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]).parse_next(input)?;
    let protected_shape = if peek((source_reference, primitives::phrase(&["this", "turn"])))
        .parse_next(input)
        .is_ok()
    {
        source_reference.parse_next(input)?;
        None
    } else {
        Some(one_or_more_tokens_before(
            input,
            primitives::phrase(&["this", "turn"]),
        )?)
    };
    primitives::phrase(&["this", "turn", "is", "dealt", "to"]).parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::NextAmount {
        amount_tokens,
        protected_tokens: protected_shape,
        destination: classify_next_amount_destination(destination_tokens),
    })
}

fn parse_redirect_next_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    alt((
        parse_all_to_you_and_permanents,
        parse_all_by_source,
        parse_all_to_target_by_choice,
        parse_next_time,
        parse_next_amount,
    ))
    .parse_next(input)
}

pub(crate) fn parse_redirect_next_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RedirectNextDamageShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_redirect_next_damage_lexed,
        "redirect next damage",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_prevent_next_damage_shape() {
        let tokens = lex(
            "Prevent the next 3 damage that would be dealt to you and permanents you control this turn by a source of your choice.",
        );
        let shape = parse_prevent_next_damage_tokens(&tokens).expect("shape");
        assert!(shape.source_of_your_choice);
        assert!(shape.protects_you_and_permanents_you_control);
    }

    #[test]
    fn parses_source_controller_redirect_without_raw_text() {
        let tokens = lex(
            "All damage that would be dealt this turn by target spell is dealt to that spell's controller instead.",
        );
        assert!(matches!(
            parse_redirect_next_damage_tokens(&tokens),
            Some(RedirectNextDamageShape::AllBySourceToSourceController { .. })
        ));
    }

    #[test]
    fn parses_next_time_redirect_target() {
        let tokens = lex(
            "The next time a red source would deal damage to target creature this turn, that damage is dealt to target player instead.",
        );
        assert!(matches!(
            parse_redirect_next_damage_tokens(&tokens),
            Some(RedirectNextDamageShape::NextTime {
                destination: RedirectDamageDestinationShape::Target(_),
                ..
            })
        ));
    }

    #[test]
    fn parses_source_object_and_chosen_destination_redirect_shapes() {
        let next_time = lex(
            "The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.",
        );
        assert!(matches!(
            parse_redirect_next_damage_tokens(&next_time),
            Some(RedirectNextDamageShape::NextTime {
                destination: RedirectDamageDestinationShape::SourceObject,
                ..
            })
        ));
        let all_damage = lex(
            "All damage that would be dealt to target creature this turn by a source of your choice is dealt to this creature instead.",
        );
        assert!(matches!(
            parse_redirect_next_damage_tokens(&all_damage),
            Some(RedirectNextDamageShape::AllToTargetByChosenSource {
                destination: RedirectDamageDestinationShape::SourceObject,
                ..
            })
        ));
        let chosen_destination = lex(
            "The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.",
        );
        assert!(matches!(
            parse_redirect_next_damage_tokens(&chosen_destination),
            Some(RedirectNextDamageShape::NextTime {
                destination: RedirectDamageDestinationShape::TargetOfChoice(_),
                ..
            })
        ));
    }
}
