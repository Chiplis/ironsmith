use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, opt, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThenSequenceShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReturnSameSubtypesShape<'a> {
    pub(crate) return_tokens: &'a [OwnedLexToken],
    pub(crate) subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseSameFilterShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseSequenceShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
    pub(crate) head_references_prior_choice: bool,
    pub(crate) tail_references_prior_choice: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReturnCreateShape<'a> {
    pub(crate) return_tokens: &'a [OwnedLexToken],
    pub(crate) create_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExileMayPutShape<'a> {
    pub(crate) exile_tokens: &'a [OwnedLexToken],
    pub(crate) put_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExileShuffleShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DestroyLandDamageShape<'a> {
    pub(crate) destroy_tokens: &'a [OwnedLexToken],
    pub(crate) damage_tokens: &'a [OwnedLexToken],
}

fn split_once<'a>(
    tokens: &'a [OwnedLexToken],
    separator: &'static [&'static str],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    primitives::split_lexed_once_on_separator(tokens, || primitives::phrase(separator).void())
        .map(|(head, tail)| (trim_lexed_commas(head), trim_lexed_commas(tail)))
        .filter(|(head, tail)| !head.is_empty() && !tail.is_empty())
}

fn exact_suffix(tokens: &[OwnedLexToken], suffix: &'static [&'static str]) -> bool {
    primitives::split_lexed_once_before_suffix(tokens, 0, || {
        (primitives::phrase(suffix), primitives::sentence_end()).void()
    })
    .is_some()
}

fn subtype_list<'a>(input: &mut LexStream<'a>) -> WResult<Vec<Subtype>> {
    repeat(
        1..,
        (
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    primitives::kw("and").void(),
                    primitives::kw("or").void(),
                    primitives::comma().void(),
                )),
            ),
            primitives::word_parser_text
                .verify_map(|word| leaf::parse_leaf_subtype_flexible_complete(word).ok()),
        )
            .map(|(_, subtype)| subtype),
    )
    .parse_next(input)
}

pub(crate) fn parse_then_sequence_shape(tokens: &[OwnedLexToken]) -> Option<ThenSequenceShape<'_>> {
    let (head_tokens, tail_tokens) = split_once(tokens, &["then"])?;
    Some(ThenSequenceShape {
        head_tokens,
        tail_tokens,
    })
}

pub(crate) fn parse_return_same_subtypes_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnSameSubtypesShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("return"))?;
    let (return_tokens, subtype_tokens) = split_once(tokens, &["do", "the", "same", "for"])?;
    let subtypes = primitives::parse_all(
        subtype_tokens,
        (subtype_list, primitives::sentence_end()).map(|(subtypes, _)| subtypes),
        "return same subtype list",
    )
    .ok()?;
    (!subtypes.is_empty()).then_some(ReturnSameSubtypesShape {
        return_tokens,
        subtypes,
    })
}

pub(crate) fn parse_choose_same_filter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseSameFilterShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("choose"))?;
    let (head_tokens, filter_tokens) = split_once(tokens, &["then", "do", "the", "same", "for"])?;
    Some(ChooseSameFilterShape {
        head_tokens,
        filter_tokens,
    })
}

pub(crate) fn parse_choice_reference_tail_shape(tokens: &[OwnedLexToken]) -> bool {
    [
        &["from", "it"][..],
        &["from", "them"][..],
        &["in", "it"][..],
        &["in", "them"][..],
    ]
    .into_iter()
    .any(|suffix| exact_suffix(tokens, suffix))
}

pub(crate) fn parse_choose_sequence_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseSequenceShape<'_>> {
    let sequence = parse_then_sequence_shape(tokens)?;
    Some(ChooseSequenceShape {
        head_tokens: sequence.head_tokens,
        tail_tokens: sequence.tail_tokens,
        head_references_prior_choice: parse_choice_reference_tail_shape(sequence.head_tokens),
        tail_references_prior_choice: parse_choice_reference_tail_shape(sequence.tail_tokens),
    })
}

pub(crate) fn parse_return_create_shape(tokens: &[OwnedLexToken]) -> Option<ReturnCreateShape<'_>> {
    let (return_tokens, create_tokens) = split_once(tokens, &["then"])?;
    let (_, return_tail) = primitives::parse_prefix(return_tokens, primitives::kw("return"))?;
    let (_, create_tail) = primitives::parse_prefix(create_tokens, primitives::kw("create"))?;
    (!trim_lexed_commas(return_tail).is_empty() && !trim_lexed_commas(create_tail).is_empty())
        .then_some(ReturnCreateShape {
            return_tokens,
            create_tokens,
        })
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

fn has_from_exile_path(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, _, tail)) = primitives::find_prefix(tokens, || primitives::kw("from")) else {
        return false;
    };
    marker_anywhere(tail, primitives::kw("exile"))
}

fn has_battlefield_path(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(
        tokens,
        (
            alt((
                primitives::kw("into"),
                primitives::kw("onto"),
                primitives::kw("to"),
            )),
            opt(alt((
                primitives::kw("a"),
                primitives::kw("an"),
                primitives::kw("the"),
            ))),
            primitives::kw("battlefield"),
        ),
    )
}

pub(crate) fn parse_exile_may_put_shape(tokens: &[OwnedLexToken]) -> Option<ExileMayPutShape<'_>> {
    let sequence = parse_then_sequence_shape(tokens)?;
    primitives::parse_prefix(
        sequence.head_tokens,
        alt((
            primitives::phrase(&["you", "exile"]).void(),
            primitives::kw("exile").void(),
        )),
    )?;
    let (_, put_body) = primitives::parse_prefix(
        sequence.tail_tokens,
        primitives::phrase(&["you", "may", "put"]),
    )?;
    let choice = leaf::parse_leaf_choice_count_prefix_tokens(put_body)?;
    let path_tokens = put_body.get(choice.consumed..)?;
    if !has_from_exile_path(path_tokens) || !has_battlefield_path(path_tokens) {
        return None;
    }
    Some(ExileMayPutShape {
        exile_tokens: sequence.head_tokens,
        put_tokens: sequence.tail_tokens,
    })
}

fn shuffle_graveyard_into_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("shuffle"), primitives::kw("shuffles"))).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("graveyard"), primitives::kw("graveyards"))),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("into"))
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("library"), primitives::kw("libraries"))),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

pub(crate) fn parse_exile_shuffle_shape(tokens: &[OwnedLexToken]) -> Option<ExileShuffleShape<'_>> {
    let sequence = parse_then_sequence_shape(tokens)?;
    alt((
        primitives::phrase(&["you", "exile"]).void(),
        primitives::kw("exile").void(),
    ))
    .parse_peek(LexStream::new(sequence.head_tokens))
    .ok()?;
    primitives::parse_all(
        sequence.tail_tokens,
        shuffle_graveyard_into_library,
        "shuffle graveyard into library",
    )
    .ok()?;
    Some(ExileShuffleShape {
        head_tokens: sequence.head_tokens,
        tail_tokens: sequence.tail_tokens,
    })
}

fn land_controller_graveyard_damage<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("deal"), primitives::kw("deals"))),
    )
    .parse_next(input)?;
    primitives::phrase(&[
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

pub(crate) fn parse_destroy_land_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyLandDamageShape<'_>> {
    let sequence = parse_then_sequence_shape(tokens)?;
    primitives::parse_prefix(sequence.head_tokens, primitives::kw("destroy"))?;
    primitives::parse_all(
        sequence.tail_tokens,
        land_controller_graveyard_damage,
        "destroyed land controller graveyard damage",
    )
    .ok()?;
    Some(DestroyLandDamageShape {
        destroy_tokens: sequence.head_tokens,
        damage_tokens: sequence.tail_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_return_same_subtypes_and_return_create() {
        let tokens = lex_line(
            "return target Elf card from your graveyard to your hand do the same for Goblins and Zombies",
            0,
        )
        .unwrap();
        let shape = parse_return_same_subtypes_shape(&tokens).expect("return-same shape");
        assert_eq!(shape.subtypes, vec![Subtype::Goblin, Subtype::Zombie]);

        let tokens = lex_line(
            "return target creature to its owners hand then create a token",
            0,
        )
        .unwrap();
        assert!(parse_return_create_shape(&tokens).is_some());
    }

    #[test]
    fn parses_exile_put_and_shuffle_sequences() {
        let tokens = lex_line(
            "you exile the top card of your library then you may put one card from exile onto the battlefield",
            0,
        )
        .unwrap();
        assert!(parse_exile_may_put_shape(&tokens).is_some());

        let tokens = lex_line(
            "exile all cards from your library then shuffle your graveyard into your library",
            0,
        )
        .unwrap();
        assert!(parse_exile_shuffle_shape(&tokens).is_some());
    }
}
