use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;
use super::super::leaf;
use crate::object::CounterType;
use crate::types::CardType;

#[path = "copy_shapes/linked_exile.rs"]
mod linked_exile;
use linked_exile::parse_linked_exile_pair_lexed;
pub use linked_exile::{LinkedExileCopyCounterValue, LinkedExilePairCopyShape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopySourceKind {
    Source,
    Enchanted,
    Filter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyExceptionDisplaySplit<'a> {
    pub before_separator: &'a [OwnedLexToken],
    pub after_separator: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnterAsCopyShape<'a> {
    LinkedExilePair(LinkedExilePairCopyShape),
    Direct {
        affected_tokens: &'a [OwnedLexToken],
        copy_source_tokens: &'a [OwnedLexToken],
        copy_source_kind: CopySourceKind,
    },
    May {
        named_subject_tokens: Option<&'a [OwnedLexToken]>,
        enters_tapped: bool,
        until_end_of_turn: bool,
        filter_tokens: &'a [OwnedLexToken],
        exception_display_split: Option<CopyExceptionDisplaySplit<'a>>,
        exception_tokens: Option<&'a [OwnedLexToken]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyCharacteristicRemainder<'a> {
    None,
    PowerToughnessFromSource,
    Abilities(&'a [OwnedLexToken]),
    /// "with crew 3 [and it loses all other card types]" (Imposter Mech).
    WithAbilities {
        abilities: &'a [OwnedLexToken],
        loses_other_card_types: bool,
    },
    /// "and it loses all other card types".
    LosesOtherCardTypes,
    ConditionalEntry(&'a [OwnedLexToken]),
    Unsupported,
}

/// "it enters with an additional +1/+1 counter on it if it's a creature".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalCopyCounterEntry {
    pub counter_type: CounterType,
    pub count: u32,
    pub card_type: CardType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyExceptionShape<'a> {
    /// Spark Double: conditional extra counters keyed on the copied source's
    /// card type, optionally with "and it isn't legendary".
    ConditionalCounters {
        entries: Vec<ConditionalCopyCounterEntry>,
        remove_legendary: bool,
    },
    Name {
        name_tokens: &'a [OwnedLexToken],
        use_named_subject: bool,
    },
    Abilities {
        ability_tokens: &'a [OwnedLexToken],
        source_filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    Characteristics {
        remove_legendary: bool,
        characteristic_tokens: &'a [OwnedLexToken],
        remainder: CopyCharacteristicRemainder<'a>,
    },
}

pub fn parse_enter_as_copy_tokens(tokens: &[OwnedLexToken]) -> Option<EnterAsCopyShape<'_>> {
    if let Ok(shape) = primitives::parse_all(
        tokens,
        parse_linked_exile_pair_lexed,
        "linked enter-as-copy",
    ) {
        return Some(EnterAsCopyShape::LinkedExilePair(shape));
    }
    if primitives::parse_prefix(tokens, primitives::phrase(&["you", "may", "have"])).is_some() {
        return crate::grammar::primitives::probe_all(
            tokens,
            parse_may_enter_as_copy_lexed,
            "may enter-as-copy",
        );
    }
    if primitives::parse_prefix(tokens, primitives::kw("as")).is_some()
        && let Ok(shape) = primitives::parse_all(
            tokens,
            parse_as_enters_become_copy_lexed,
            "temporary as-enters copy",
        )
    {
        return Some(shape);
    }
    crate::grammar::primitives::probe_all(
        tokens,
        parse_direct_enter_as_copy_lexed,
        "direct enter-as-copy",
    )
}

pub fn parse_copy_exception_tokens(tokens: &[OwnedLexToken]) -> Option<CopyExceptionShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_copy_conditional_counters_exception_lexed,
            parse_copy_name_exception_lexed,
            parse_copy_ability_exception_lexed,
            parse_copy_characteristic_exception_lexed,
        )),
        "enter-as-copy exception",
    )
}

fn conditional_copy_counter_entry<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalCopyCounterEntry> {
    primitives::phrase(&["it", "enters", "with"]).parse_next(input)?;
    let count = alt((
        alt((primitives::kw("a"), primitives::kw("an"))).value(1u32),
        leaf::parse_leaf_number_prefix_lexed,
    ))
    .parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    let counter_type = any
        .verify_map(|token: &OwnedLexToken| {
            crate::util::parse_counter_type_word(token.parser_text())
        })
        .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["on", "it", "if"]).parse_next(input)?;
    alt((
        (primitives::kw("it"), primitives::kw("s")).void(),
        primitives::kw("it's").void(),
        primitives::kw("its").void(),
        (primitives::kw("it"), primitives::kw("is")).void(),
    ))
    .parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let card_type = any
        .verify_map(|token: &OwnedLexToken| {
            leaf::parse_leaf_card_type_complete(token.parser_text()).ok()
        })
        .parse_next(input)?;
    Ok(ConditionalCopyCounterEntry {
        counter_type,
        count,
        card_type,
    })
}

fn parse_copy_conditional_counters_exception_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CopyExceptionShape<'a>> {
    let mut entries = Vec::new();
    let mut remove_legendary = false;
    loop {
        if let Some(entry) = opt(conditional_copy_counter_entry).parse_next(input)? {
            entries.push(entry);
        } else if opt(alt((
            primitives::phrase(&["it", "isn't", "legendary"]),
            primitives::phrase(&["it", "isnt", "legendary"]),
            primitives::phrase(&["it", "is", "not", "legendary"]),
        )))
        .parse_next(input)?
        .is_some()
        {
            remove_legendary = true;
        } else {
            return Err(primitives::backtrack_err(
                "copy exception",
                "conditional counter entry",
            ));
        }
        if peek(primitives::sentence_end()).parse_next(input).is_ok() {
            primitives::sentence_end().parse_next(input)?;
            break;
        }
        opt(primitives::comma()).parse_next(input)?;
        opt(primitives::kw("and")).parse_next(input)?;
    }
    if entries.is_empty() {
        return Err(primitives::backtrack_err(
            "copy exception",
            "at least one conditional counter entry",
        ));
    }
    Ok(CopyExceptionShape::ConditionalCounters {
        entries,
        remove_legendary,
    })
}

fn parse_direct_enter_as_copy_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EnterAsCopyShape<'a>> {
    let affected_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("enter"), primitives::kw("enters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("enter"), primitives::kw("enters"))).parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    primitives::phrase(&["as", "a", "copy", "of"]).parse_next(input)?;
    let copy_source_tokens = take_sentence_body(input)?;
    let copy_source_kind = if primitives::parse_prefix(copy_source_tokens, primitives::kw("this"))
        .is_some()
    {
        CopySourceKind::Source
    } else if primitives::parse_prefix(copy_source_tokens, primitives::kw("enchanted")).is_some() {
        CopySourceKind::Enchanted
    } else {
        CopySourceKind::Filter
    };
    Ok(EnterAsCopyShape::Direct {
        affected_tokens: trim_lexed_commas(affected_tokens),
        copy_source_tokens: trim_lexed_commas(copy_source_tokens),
        copy_source_kind,
    })
}

fn parse_may_enter_as_copy_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EnterAsCopyShape<'a>> {
    let full_len = input.eof_offset();
    let full_tokens = input.peek_slice(full_len);
    primitives::phrase(&["you", "may", "have"]).parse_next(input)?;
    let subject_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("enter"), primitives::kw("enters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("enter"), primitives::kw("enters"))).parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    let enters_tapped = opt(primitives::kw("tapped"))
        .map(|parsed| parsed.is_some())
        .parse_next(input)?;
    primitives::phrase(&["as", "a", "copy", "of"]).parse_next(input)?;
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            (opt(primitives::comma()), primitives::kw("except")).void(),
            primitives::sentence_end(),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let (exception_display_split, exception_tokens) =
        if peek((opt(primitives::comma()), primitives::kw("except")))
            .parse_next(input)
            .is_ok()
        {
            let separator_len = input.eof_offset();
            let separator_suffix = input.peek_slice(separator_len);
            let separator_comma = opt(primitives::comma()).parse_next(input)?;
            let after_separator_len = input.eof_offset();
            let after_separator = input.peek_slice(after_separator_len);
            let display_split = separator_comma.map(|_| CopyExceptionDisplaySplit {
                before_separator: &full_tokens
                    [..full_tokens.len().saturating_sub(separator_suffix.len())],
                after_separator,
            });
            primitives::kw("except").parse_next(input)?;
            (display_split, Some(take_sentence_body(input)?))
        } else {
            primitives::sentence_end().parse_next(input)?;
            (None, None)
        };
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let named_subject_tokens =
        if primitives::parse_prefix(subject_tokens, primitives::kw("this")).is_some() {
            None
        } else {
            Some(subject_tokens)
        };
    Ok(EnterAsCopyShape::May {
        named_subject_tokens,
        enters_tapped,
        until_end_of_turn: false,
        filter_tokens: trim_lexed_commas(filter_tokens),
        exception_display_split,
        exception_tokens: exception_tokens.map(trim_lexed_commas),
    })
}

/// Parse the replacement-effect wording used by temporary copy permanents:
/// "As this artifact enters, you may have it become a copy of ... until end
/// of turn, except ...". This is still an as-enters copy choice; the duration
/// changes only whether the entering object's underlying copiable values are
/// replaced permanently.
fn parse_as_enters_become_copy_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EnterAsCopyShape<'a>> {
    primitives::kw("as").parse_next(input)?;
    let subject_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("enter"), primitives::kw("enters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("enter"), primitives::kw("enters"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "have", "it", "become", "a", "copy", "of"])
        .parse_next(input)?;
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["until", "end", "of", "turn"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;

    let exception_tokens = if peek((opt(primitives::comma()), primitives::kw("except")))
        .parse_next(input)
        .is_ok()
    {
        opt(primitives::comma()).parse_next(input)?;
        primitives::kw("except").parse_next(input)?;
        Some(take_sentence_body(input)?)
    } else {
        primitives::sentence_end().parse_next(input)?;
        None
    };

    let subject_tokens = trim_lexed_commas(subject_tokens);
    let named_subject_tokens =
        if primitives::parse_prefix(subject_tokens, primitives::kw("this")).is_some() {
            None
        } else {
            Some(subject_tokens)
        };
    Ok(EnterAsCopyShape::May {
        named_subject_tokens,
        enters_tapped: false,
        until_end_of_turn: true,
        filter_tokens: trim_lexed_commas(filter_tokens),
        exception_display_split: None,
        exception_tokens: exception_tokens.map(trim_lexed_commas),
    })
}

fn parse_copy_name_exception_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CopyExceptionShape<'a>> {
    alt((
        primitives::phrase(&["its", "name", "is"]),
        primitives::phrase(&["his", "name", "is"]),
        primitives::phrase(&["her", "name", "is"]),
    ))
    .parse_next(input)?;
    let name_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            primitives::kw("and").void(),
            primitives::kw("it's").void(),
            primitives::kw("it’s").void(),
            primitives::sentence_end(),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end()))
        .void()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CopyExceptionShape::Name {
        name_tokens: trim_lexed_commas(name_tokens),
        use_named_subject: primitives::parse_prefix(name_tokens, primitives::kw("this")).is_some(),
    })
}

fn parse_copy_ability_exception_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CopyExceptionShape<'a>> {
    primitives::phrase(&["it", "has"]).parse_next(input)?;
    let ability_tokens = take_sentence_body(input)?;
    let (ability_tokens, source_filter_tokens) =
        match primitives::split_lexed_once_on_separator(ability_tokens, || {
            primitives::phrase(&["if", "that"]).void()
        }) {
            Some((abilities, filter)) => (abilities, Some(filter)),
            None => (ability_tokens, None),
        };
    Ok(CopyExceptionShape::Abilities {
        ability_tokens: trim_lexed_commas(ability_tokens),
        source_filter_tokens,
    })
}

fn parse_copy_characteristic_exception_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CopyExceptionShape<'a>> {
    let remove_legendary = opt(alt((
        primitives::phrase(&["it", "isn't", "legendary"]),
        primitives::phrase(&["it", "isnt", "legendary"]),
        primitives::phrase(&["it", "is", "not", "legendary"]),
    )))
    .map(|parsed| parsed.is_some())
    .parse_next(input)?;
    if remove_legendary {
        opt(primitives::comma()).parse_next(input)?;
    }
    alt((
        (
            primitives::kw("its"),
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
        (
            primitives::kw("is"),
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
        (
            primitives::kw("it"),
            alt((primitives::kw("is"), primitives::kw("s"))),
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
        (
            primitives::kw("it's"),
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
    ))
    .parse_next(input)?;
    let characteristic_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            parse_copy_characteristic_remainder_start,
            primitives::sentence_end(),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let remainder = if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        CopyCharacteristicRemainder::None
    } else {
        parse_copy_characteristic_remainder(input)?
    };
    Ok(CopyExceptionShape::Characteristics {
        remove_legendary,
        characteristic_tokens: trim_lexed_commas(characteristic_tokens),
        remainder,
    })
}

fn parse_copy_characteristic_remainder_start<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&[
            "in", "addition", "to", "its", "other", "colors", "and", "types",
        ]),
        primitives::phrase(&["in", "addition", "to", "its", "other", "types"]),
        primitives::phrase(&["in", "addition", "to", "its", "other", "creature", "types"]),
        primitives::phrase(&["and", "its", "power", "and", "toughness"]),
        primitives::phrase(&["its", "power", "and", "toughness"]),
        primitives::phrase(&["and", "it", "has"]),
        primitives::phrase(&["and", "has"]),
        primitives::kw("with").void(),
        primitives::phrase(&["and", "it", "loses", "all", "other", "card", "types"]),
    ))
    .void()
    .parse_next(input)
}

fn loses_other_card_types_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::phrase(&["and", "it", "loses", "all", "other", "card", "types"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn parse_copy_characteristic_remainder<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CopyCharacteristicRemainder<'a>> {
    opt(alt((
        primitives::phrase(&[
            "in", "addition", "to", "its", "other", "colors", "and", "types",
        ]),
        primitives::phrase(&["in", "addition", "to", "its", "other", "types"]),
        primitives::phrase(&["in", "addition", "to", "its", "other", "creature", "types"]),
    )))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        return Ok(CopyCharacteristicRemainder::None);
    }
    alt((
        (
            primitives::kw("with"),
            repeat_till::<_, _, (), _, _, _, _>(
                1..,
                any.void(),
                peek(alt((loses_other_card_types_tail, primitives::sentence_end().void()))),
            )
            .take(),
            opt(loses_other_card_types_tail),
            opt(primitives::sentence_end()),
        )
            .map(|(_, abilities, loses, _)| CopyCharacteristicRemainder::WithAbilities {
                abilities: trim_lexed_commas(abilities),
                loses_other_card_types: loses.is_some(),
            }),
        loses_other_card_types_tail.value(CopyCharacteristicRemainder::LosesOtherCardTypes),
        (
            primitives::kw("and"),
            opt(primitives::comma()),
            primitives::kw("if"),
            take_sentence_body,
        )
            .map(|(_, _, _, entry)| CopyCharacteristicRemainder::ConditionalEntry(entry)),
        (
            opt(primitives::kw("and")),
            primitives::phrase(&[
                "its",
                "power",
                "and",
                "toughness",
                "are",
                "equal",
                "to",
                "this",
            ]),
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end())),
            primitives::sentence_end(),
        )
            .value(CopyCharacteristicRemainder::PowerToughnessFromSource),
        (
            primitives::kw("and"),
            opt(primitives::kw("it")),
            primitives::kw("has"),
            take_sentence_body,
        )
            .map(|(_, _, _, abilities)| {
                CopyCharacteristicRemainder::Abilities(trim_lexed_commas(abilities))
            }),
        (
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end())),
            primitives::sentence_end(),
        )
            .value(CopyCharacteristicRemainder::Unsupported),
    ))
    .parse_next(input)
}

fn take_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(trim_lexed_commas(tokens))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_direct_and_may_copy_shapes() {
        let tokens =
            lex_line("Creatures you control enter as a copy of this creature.", 0).unwrap();
        assert!(matches!(
            parse_enter_as_copy_tokens(&tokens),
            Some(EnterAsCopyShape::Direct {
                copy_source_kind: CopySourceKind::Source,
                ..
            })
        ));
        let tokens = lex_line(
            "You may have this creature enter as a copy of a creature you control, except it has flying.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_enter_as_copy_tokens(&tokens),
            Some(EnterAsCopyShape::May {
                exception_tokens: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn parses_completed_type_and_subtype_addition_exceptions() {
        for (text, expected_word) in [
            (
                "it's an artifact in addition to its other types",
                "artifact",
            ),
            (
                "it's a Ninja in addition to its other creature types",
                "ninja",
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let Some(CopyExceptionShape::Characteristics {
                characteristic_tokens,
                remainder,
                ..
            }) = parse_copy_exception_tokens(&tokens)
            else {
                panic!("expected typed characteristic exception for {text:?}");
            };
            assert_eq!(
                super::super::super::super::lexer::parser_token_word_refs(characteristic_tokens),
                [expected_word]
            );
            assert_eq!(remainder, CopyCharacteristicRemainder::None);
        }
    }

    #[test]
    fn parses_nonlegendary_type_and_ability_exception() {
        let tokens = lex_line(
            "it isn't legendary, is an artifact in addition to its other types, and has myriad",
            0,
        )
        .unwrap();
        let Some(CopyExceptionShape::Characteristics {
            remove_legendary,
            remainder: CopyCharacteristicRemainder::Abilities(ability_tokens),
            ..
        }) = parse_copy_exception_tokens(&tokens)
        else {
            panic!("expected typed nonlegendary artifact-and-ability exception");
        };
        assert!(remove_legendary);
        assert_eq!(
            super::super::super::super::lexer::parser_token_word_refs(ability_tokens),
            ["myriad"]
        );
    }
}

pub fn split_copy_source_missing_ability_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    primitives::split_lexed_once_on_separator(tokens, || {
        primitives::any_phrase(&[
            &["doesnt", "have"],
            &["doesn't", "have"],
            &["does", "not", "have"],
        ])
        .void()
    })
}
