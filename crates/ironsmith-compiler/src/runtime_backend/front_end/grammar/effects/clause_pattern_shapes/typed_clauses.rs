use crate::cards::builders::OwnedLexToken;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, trim_lexed_commas};
use winnow::Parser as _;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseTargetVerbShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreventAllDamageSourceShape<'a> {
    Choice,
    ChoiceSharingActivationManaColor,
    Filter(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreventAllDamageShape<'a> {
    FromSource {
        source_tokens: &'a [OwnedLexToken],
    },
    ToTarget {
        target_tokens: &'a [OwnedLexToken],
    },
    ToTargetFromSource {
        target_tokens: &'a [OwnedLexToken],
        source: PreventAllDamageSourceShape<'a>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseTargetPreludeShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

fn effect_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("add"),
            primitives::kw("move"),
            primitives::kw("counter"),
            primitives::kw("destroy"),
            primitives::kw("exile"),
            primitives::kw("draw"),
            primitives::kw("deal"),
            primitives::kw("sacrifice"),
        ))
        .void(),
        alt((
            primitives::kw("create"),
            primitives::kw("investigate"),
            primitives::kw("proliferate"),
            primitives::kw("tap"),
            primitives::kw("attach"),
            primitives::kw("untap"),
            primitives::kw("scry"),
            primitives::kw("discard"),
        ))
        .void(),
        alt((
            primitives::kw("transform"),
            primitives::kw("convert"),
            primitives::kw("regenerate"),
            primitives::kw("mill"),
        ))
        .void(),
        alt((
            primitives::kw("get"),
            primitives::kw("remove"),
            primitives::kw("return"),
            primitives::kw("exchange"),
            primitives::kw("become"),
            primitives::kw("skip"),
            primitives::kw("surveil"),
            primitives::kw("incubate"),
        ))
        .void(),
        alt((
            alt((
                primitives::kw("shuffle"),
                primitives::kw("pay"),
                primitives::kw("detain"),
                primitives::kw("goad"),
                primitives::kw("suspect"),
                primitives::kw("note"),
                primitives::kw("look"),
            )),
            alt((
                primitives::kw("roll"),
                primitives::kw("flip"),
                primitives::kw("end"),
            )),
        ))
        .void(),
    ))
    .parse_next(input)
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

fn demonstrative_object_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("that"), primitives::kw("those"))).parse_next(input)?;
    alt((
        alt((
            primitives::kw("creature"),
            primitives::kw("creatures"),
            primitives::kw("permanent"),
            primitives::kw("permanents"),
            primitives::kw("artifact"),
            primitives::kw("artifacts"),
            primitives::kw("enchantment"),
            primitives::kw("enchantments"),
        )),
        alt((
            primitives::kw("land"),
            primitives::kw("lands"),
            primitives::kw("card"),
            primitives::kw("cards"),
            primitives::kw("token"),
            primitives::kw("tokens"),
            primitives::kw("spell"),
            primitives::kw("spells"),
        )),
    ))
    .void()
    .parse_next(input)
}

fn simple_chosen_object_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("then")).parse_next(input)?;
    opt(article).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::kw("them").void(),
        demonstrative_object_reference,
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_choose_target_verb_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ChooseTargetVerbShape<'a>> {
    primitives::kw("choose").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::kw("and")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("then")).parse_next(input)?;
    let action_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let mut action = LexStream::new(action_tokens);
    effect_verb.parse_next(&mut action)?;
    simple_chosen_object_reference.parse_next(&mut action)?;

    let target_tokens = trim_lexed_commas(target_tokens);
    let mut target = LexStream::new(target_tokens);
    primitives::kw("target").parse_next(&mut target)?;
    if marker_present(target_tokens, effect_verb) {
        return Err(primitives::backtrack_err(
            "choose target and action",
            "target phrase without effect verb",
        ));
    }

    Ok(ChooseTargetVerbShape {
        target_tokens,
        action_tokens: trim_lexed_commas(action_tokens),
    })
}

pub(crate) fn parse_choose_target_verb_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChooseTargetVerbShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_choose_target_verb_lexed,
        "choose target and action",
    )
    .ok()
}

fn source_descriptor<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let descriptor = repeat_till(
        1..,
        any.void(),
        peek((opt(primitives::kw("sources")), primitives::sentence_end())),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::kw("sources")).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(trim_lexed_commas(descriptor))
}

fn source_of_your_choice<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    opt(article).parse_next(input)?;
    primitives::phrase(&["source", "of", "your", "choice"]).parse_next(input)?;
    let shares_activation_mana_color = opt(primitives::phrase(&[
        "that", "shares", "a", "color", "with",
    ]))
    .parse_next(input)?
    .is_some();
    if shares_activation_mana_color {
        opt(primitives::kw("the")).parse_next(input)?;
        primitives::phrase(&["mana", "spent", "on", "this", "activation", "cost"])
            .parse_next(input)?;
    }
    primitives::sentence_end().parse_next(input)?;
    Ok(shares_activation_mana_color)
}

fn classify_prevent_source(tokens: &[OwnedLexToken]) -> PreventAllDamageSourceShape<'_> {
    match primitives::parse_all(tokens, source_of_your_choice, "source of your choice") {
        Ok(true) => PreventAllDamageSourceShape::ChoiceSharingActivationManaColor,
        Ok(false) => PreventAllDamageSourceShape::Choice,
        Err(_) => PreventAllDamageSourceShape::Filter(trim_lexed_commas(tokens)),
    }
}

fn parse_duration_first_source<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventAllDamageShape<'a>> {
    primitives::phrase(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "by",
    ])
    .parse_next(input)?;
    Ok(PreventAllDamageShape::FromSource {
        source_tokens: source_descriptor.parse_next(input)?,
    })
}

fn parse_duration_first_target<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventAllDamageShape<'a>> {
    primitives::phrase(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ])
    .parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PreventAllDamageShape::ToTarget {
        target_tokens: trim_lexed_commas(target_tokens),
    })
}

fn parse_target_first_source<'a>(input: &mut LexStream<'a>) -> WResult<PreventAllDamageShape<'a>> {
    primitives::phrase(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ])
    .parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["this", "turn", "by"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["this", "turn", "by"]).parse_next(input)?;
    let source_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PreventAllDamageShape::ToTargetFromSource {
        target_tokens: trim_lexed_commas(target_tokens),
        source: classify_prevent_source(source_tokens),
    })
}

fn parse_target_source_duration<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventAllDamageShape<'a>> {
    primitives::phrase(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ])
    .parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::kw("by")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("by").parse_next(input)?;
    let source_tokens = repeat_till(1.., any.void(), peek(primitives::phrase(&["this", "turn"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PreventAllDamageShape::ToTargetFromSource {
        target_tokens: trim_lexed_commas(target_tokens),
        source: classify_prevent_source(source_tokens),
    })
}

fn parse_target_first<'a>(input: &mut LexStream<'a>) -> WResult<PreventAllDamageShape<'a>> {
    primitives::phrase(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ])
    .parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek((
            primitives::phrase(&["this", "turn"]),
            primitives::sentence_end(),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PreventAllDamageShape::ToTarget {
        target_tokens: trim_lexed_commas(target_tokens),
    })
}

pub(crate) fn parse_prevent_all_damage_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreventAllDamageShape<'_>> {
    primitives::parse_all(
        tokens,
        alt((
            parse_duration_first_source,
            parse_duration_first_target,
            parse_target_source_duration,
            parse_target_first_source,
            parse_target_first,
        )),
        "prevent all damage",
    )
    .ok()
}

fn marker_present<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn has_no_defender_tail(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("have")),
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("defender"))),
            primitives::kw("defender"),
            primitives::sentence_end(),
        )
            .void(),
        "as though no defender",
    )
    .is_ok()
        || primitives::parse_all(
            tokens,
            (
                repeat_till::<_, _, (), _, _, _, _>(
                    0..,
                    any.void(),
                    peek(alt((primitives::kw("didnt"), primitives::kw("didn't")))),
                ),
                alt((primitives::kw("didnt"), primitives::kw("didn't"))),
                primitives::sentence_end(),
            )
                .void(),
            "as though did not",
        )
        .is_ok()
}

fn parse_can_attack_no_defender_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    let subject_tokens = repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&["can", "attack"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["can", "attack"]).parse_next(input)?;
    let tail = input.as_ref();
    if !marker_present(tail, primitives::phrase(&["as", "though"]))
        || !has_no_defender_tail(tail)
    {
        return Err(primitives::backtrack_err(
            "attack as though",
            "turn and no-defender phrase",
        ));
    }
    while any::<_, ErrMode<ContextError>>.parse_next(input).is_ok() {}
    Ok(trim_lexed_commas(subject_tokens))
}

pub(crate) fn parse_can_attack_no_defender_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_can_attack_no_defender_lexed,
        "can attack as though no defender",
    )
    .ok()
}

fn target_indicator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::phrase(&["any", "number", "of"])).parse_next(input)?;
    let mut count_probe = input.clone();
    if crate::runtime_backend::front_end::grammar::leaf::parse_leaf_target_count_range_prefix_lexed
        .parse_next(&mut count_probe)
        .is_ok()
        || crate::runtime_backend::front_end::grammar::leaf::parse_leaf_choice_count_prefix_lexed
            .parse_next(&mut count_probe)
            .is_ok()
    {
        *input = count_probe;
    } else {
        let mut fixed_probe = input.clone();
        if crate::runtime_backend::front_end::grammar::leaf::parse_leaf_number_prefix_lexed
            .parse_next(&mut fixed_probe)
            .is_ok()
            && peek(primitives::kw("target"))
                .parse_next(&mut fixed_probe)
                .is_ok()
        {
            *input = fixed_probe;
        } else {
            opt(primitives::kw("x")).parse_next(input)?;
        }
    }
    opt(primitives::kw("on")).parse_next(input)?;
    opt(primitives::kw("another")).parse_next(input)?;
    primitives::kw("target").void().parse_next(input)
}

fn parse_choose_target_prelude_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ChooseTargetPreludeShape<'a>> {
    primitives::kw("choose").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let trimmed = trim_lexed_commas(target_tokens);
    let mut target_probe = LexStream::new(trimmed);
    target_indicator.parse_next(&mut target_probe)?;
    if marker_present(trimmed, effect_verb) {
        return Err(primitives::backtrack_err(
            "choose target prelude",
            "target phrase without effect verb",
        ));
    }
    Ok(ChooseTargetPreludeShape {
        target_tokens: trimmed,
    })
}

pub(crate) fn parse_choose_target_prelude_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChooseTargetPreludeShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_choose_target_prelude_lexed,
        "choose target prelude",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_choose_and_prevent_shapes() {
        let choose = lex_line("Choose target creature and tap it.", 0).unwrap();
        let parsed = parse_choose_target_verb_shape_tokens(&choose).unwrap();
        assert!(!parsed.target_tokens.is_empty());

        let prevent = lex_line(
            "Prevent all damage that would be dealt to you this turn by a source of your choice.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_prevent_all_damage_shape_tokens(&prevent),
            Some(PreventAllDamageShape::ToTargetFromSource {
                source: PreventAllDamageSourceShape::Choice,
                ..
            })
        ));

        let color_limited = lex_line(
            "Prevent all damage that would be dealt to you this turn by a source of your choice that shares a color with the mana spent on this activation cost.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_prevent_all_damage_shape_tokens(&color_limited),
            Some(PreventAllDamageShape::ToTargetFromSource {
                source: PreventAllDamageSourceShape::ChoiceSharingActivationManaColor,
                ..
            })
        ));
    }

    #[test]
    fn parses_attack_and_target_prelude_shapes() {
        let attack = lex_line(
            "Target creature can attack this turn as though it didn't have defender.",
            0,
        )
        .unwrap();
        assert!(parse_can_attack_no_defender_subject_tokens(&attack).is_some());

        let prelude = lex_line("Choose up to two target creatures.", 0).unwrap();
        assert!(parse_choose_target_prelude_shape_tokens(&prelude).is_some());

        let compound = lex_line(
            "Choose target instant or sorcery card in your graveyard, then roll a d20.",
            0,
        )
        .unwrap();
        assert!(parse_choose_target_prelude_shape_tokens(&compound).is_none());

        let coin_flip = lex_line("Choose target spell, then flip a coin.", 0).unwrap();
        assert!(parse_choose_target_prelude_shape_tokens(&coin_flip).is_none());
    }
}
