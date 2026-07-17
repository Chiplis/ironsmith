use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DoubleCounterHolderShape<'a> {
    You,
    Source {
        tokens: &'a [OwnedLexToken],
        surface: crate::target::SourceReferenceSurface,
    },
    Target(&'a [OwnedLexToken]),
    Filter(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoubleCountersShape<'a> {
    pub(crate) counter_type: Option<crate::object::CounterType>,
    pub(crate) holder: DoubleCounterHolderShape<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CopyTailShape {
    pub(crate) retarget_split: Option<usize>,
    pub(crate) retarget_may: bool,
    pub(crate) retarget_single_target: bool,
    pub(crate) exception_split: Option<usize>,
    pub(crate) then_split: Option<usize>,
    pub(crate) for_each_split: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyClauseShape {
    pub(crate) copy_word: usize,
    pub(crate) exception_word: Option<usize>,
    pub(crate) emblem_with: bool,
    pub(crate) simple_reference: bool,
    pub(crate) mentions_spell_or_ability: bool,
    pub(crate) removed_legendary: bool,
    pub(crate) tail: CopyTailShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyTargetShape<'a> {
    Source,
    Triggering,
    TriggeringSource,
    TaggedIt,
    Explicit(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyRetargetShape {
    pub(crate) may_choose: bool,
    pub(crate) has_new: bool,
    pub(crate) single_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanBlockAdditionalShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) additional: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WinGameShape<'a> {
    Simple,
    ConditionalTail,
    NamedZones { name_tokens: &'a [OwnedLexToken] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KickedAdditionalTargetsShape<'a> {
    pub(crate) first_target_tokens: &'a [OwnedLexToken],
    pub(crate) additional_target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConniveSubjectShape<'a> {
    ConvokedThisSpell,
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConniveClauseShape<'a> {
    pub(crate) subject: ConniveSubjectShape<'a>,
    pub(crate) count_tokens: &'a [OwnedLexToken],
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

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn double_counter_source_surface(
    tokens: &[OwnedLexToken],
) -> Option<crate::target::SourceReferenceSurface> {
    let words = parser_token_word_refs(tokens);
    if let Some(anaphor) = leaf::parse_leaf_source_anaphor_words(&words) {
        return Some(match anaphor {
            leaf::LeafSourceAnaphor::It => {
                crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string())
            }
            leaf::LeafSourceAnaphor::Its => {
                crate::target::SourceReferenceSurface::ThisPermanentType("its".to_string())
            }
            leaf::LeafSourceAnaphor::This(surface) => surface,
        });
    }
    crate::runtime_backend::util::source_reference_surface_for_words(&words)
}

fn is_singular_typed_demonstrative(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    matches!(
        words.as_slice(),
        ["that", head] if crate::runtime_backend::util::is_demonstrative_object_head(head)
    )
}

fn parse_double_counters_lexed<'a>(input: &mut LexStream<'a>) -> WResult<DoubleCountersShape<'a>> {
    primitives::phrase(&["double", "the", "number", "of"]).parse_next(input)?;
    let counter_phrase_tokens = (
        repeat_till(1.., any.void(), peek(counter_noun)).map(|((), _)| ()),
        counter_noun,
    )
        .take()
        .parse_next(input)?;
    let counter_type = if primitives::parse_all(
        counter_phrase_tokens,
        (
            primitives::phrase(&["each", "kind", "of"]),
            counter_noun,
            eof,
        )
            .void(),
        "each kind of counter",
    )
    .is_ok()
    {
        None
    } else {
        Some(
            crate::runtime_backend::front_end::grammar::filters::parse_counter_type_from_tokens(
                counter_phrase_tokens,
            )
            .ok_or_else(|| {
                primitives::backtrack_err("double counters", "recognized counter type")
            })?,
        )
    };

    let holder = if primitives::parse_prefix(
        input.as_ref(),
        (
            primitives::phrase(&["you", "have"]),
            primitives::sentence_end(),
        )
            .void(),
    )
    .is_some()
    {
        primitives::phrase(&["you", "have"]).parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        DoubleCounterHolderShape::You
    } else {
        primitives::kw("on").parse_next(input)?;
        let has_explicit_collection_quantifier =
            opt(alt((primitives::kw("each"), primitives::kw("all"))))
                .parse_next(input)?
                .is_some();
        let holder_tokens =
            repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
                .map(|((), _)| ())
                .take()
                .parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        if let Some(surface) = double_counter_source_surface(holder_tokens) {
            DoubleCounterHolderShape::Source {
                tokens: holder_tokens,
                surface,
            }
        } else if !has_explicit_collection_quantifier
            && (marker_anywhere(
                holder_tokens,
                alt((primitives::kw("target"), primitives::kw("targets"))),
            ) || is_singular_typed_demonstrative(holder_tokens))
        {
            DoubleCounterHolderShape::Target(holder_tokens)
        } else {
            DoubleCounterHolderShape::Filter(holder_tokens)
        }
    };
    Ok(DoubleCountersShape {
        counter_type,
        holder,
    })
}

pub(crate) fn parse_double_counters_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DoubleCountersShape<'_>> {
    primitives::parse_all(tokens, parse_double_counters_lexed, "double counters").ok()
}

#[path = "utility/copy_shapes.rs"]
mod copy_shapes;
pub(crate) use copy_shapes::*;

pub(crate) fn has_counter_ability_markers_tokens(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(tokens, primitives::kw("ability"))
        && marker_anywhere(
            tokens,
            alt((primitives::kw("activated"), primitives::kw("triggered"))),
        )
}

fn parse_can_block_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CanBlockAdditionalShape<'a>> {
    let subject_tokens = repeat_till(0.., any.void(), peek(primitives::phrase(&["can", "block"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["can", "block"]).parse_next(input)?;
    let count_tokens = repeat_till(0.., any.void(), peek(primitives::kw("additional")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&["this", "turn"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let additional = count_tokens
        .last()
        .and_then(|token| {
            primitives::parse_all(
                std::slice::from_ref(token),
                (leaf::parse_leaf_number_prefix_lexed, eof).map(|(count, _)| count),
                "additional blocker count",
            )
            .ok()
        })
        .unwrap_or(1);
    Ok(CanBlockAdditionalShape {
        subject_tokens,
        additional,
    })
}

pub(crate) fn parse_can_block_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CanBlockAdditionalShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_can_block_additional_lexed,
        "can block additional creature",
    )
    .ok()
}

fn zone_markers<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    let remaining = input.as_ref();
    for expected in ["exile", "hand", "graveyard", "battlefield"] {
        if !marker_anywhere(remaining, primitives::kw(expected)) {
            return Err(primitives::backtrack_err(
                "win-game zones",
                "all required zones",
            ));
        }
    }
    while any::<_, ErrMode<ContextError>>.parse_next(input).is_ok() {}
    Ok(())
}

fn parse_win_game_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WinGameShape<'a>> {
    primitives::phrase(&["you", "win", "the", "game"]).parse_next(input)?;
    if primitives::sentence_end()
        .parse_next(&mut input.clone())
        .is_ok()
    {
        primitives::sentence_end().parse_next(input)?;
        return Ok(WinGameShape::Simple);
    }
    primitives::kw("if").parse_next(input)?;
    let has_conditional_tail = !input.as_ref().is_empty();
    let mut named_probe = input.clone();
    if primitives::phrase(&["you", "own"])
        .parse_next(&mut named_probe)
        .is_ok()
        && opt(alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        )))
        .parse_next(&mut named_probe)
        .is_ok()
        && primitives::phrase(&["card", "named"])
            .parse_next(&mut named_probe)
            .is_ok()
    {
        let name_tokens = repeat_till(1.., any.void(), peek(primitives::kw("in")))
            .map(|((), _)| ())
            .take()
            .parse_next(&mut named_probe)?;
        primitives::kw("in").parse_next(&mut named_probe)?;
        zone_markers.parse_next(&mut named_probe)?;
        *input = named_probe;
        return Ok(WinGameShape::NamedZones { name_tokens });
    }
    while any::<_, ErrMode<ContextError>>.parse_next(input).is_ok() {}
    has_conditional_tail
        .then_some(WinGameShape::ConditionalTail)
        .ok_or_else(|| primitives::backtrack_err("win-game condition", "condition"))
}

pub(crate) fn parse_win_game_shape_tokens(tokens: &[OwnedLexToken]) -> Option<WinGameShape<'_>> {
    primitives::parse_all(tokens, parse_win_game_lexed, "win the game").ok()
}

fn canonical_target_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| !matches!(*word, "another" | "other" | "target" | "a" | "an" | "the"))
        .collect()
}

fn parse_kicked_targets_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KickedAdditionalTargetsShape<'a>> {
    let first_target_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["then", "choose"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["then", "choose"]).parse_next(input)?;
    let additional_target_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&[
            "for", "each", "time", "this", "spell", "was", "kicked",
        ])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["for", "each", "time", "this", "spell", "was", "kicked"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if canonical_target_words(first_target_tokens)
        != canonical_target_words(additional_target_tokens)
    {
        return Err(primitives::backtrack_err(
            "kicked targets",
            "matching target descriptions",
        ));
    }
    Ok(KickedAdditionalTargetsShape {
        first_target_tokens,
        additional_target_tokens,
    })
}

pub(crate) fn parse_kicked_additional_targets_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KickedAdditionalTargetsShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_kicked_targets_lexed,
        "kicked additional targets",
    )
    .ok()
}

fn connive_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("connive"), primitives::kw("connives")))
        .void()
        .parse_next(input)
}

fn convoked_this_spell_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("each")).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    primitives::phrase(&["that", "convoked", "this", "spell"]).parse_next(input)?;
    eof.void().parse_next(input)
}

fn parse_connive_clause_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ConniveClauseShape<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(connive_verb))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    connive_verb.parse_next(input)?;
    let count_tokens = repeat_till(0.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let subject = if primitives::parse_all(
        subject_tokens,
        convoked_this_spell_subject,
        "convoked creatures",
    )
    .is_ok()
    {
        ConniveSubjectShape::ConvokedThisSpell
    } else if let Some((_, target_tokens)) =
        primitives::parse_prefix(subject_tokens, primitives::phrase(&["each", "of"]))
        && primitives::parse_prefix(target_tokens, primitives::kw("x")).is_some()
    {
        ConniveSubjectShape::Target(target_tokens)
    } else {
        ConniveSubjectShape::Target(subject_tokens)
    };
    Ok(ConniveClauseShape {
        subject,
        count_tokens,
    })
}

pub(crate) fn parse_connive_clause_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConniveClauseShape<'_>> {
    primitives::parse_all(tokens, parse_connive_clause_lexed, "connive clause").ok()
}

#[cfg(test)]
#[path = "utility/tests.rs"]
mod tests;
