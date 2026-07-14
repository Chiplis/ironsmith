use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::ChoiceCount;
use crate::target::PlayerFilter;
use crate::types::CardType;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use super::{leaf, primitives};

#[path = "choices/object_shapes.rs"]
mod object_shapes;
pub(crate) use object_shapes::*;

#[path = "choices/typed_object_filters.rs"]
mod typed_object_filters;
pub(crate) use typed_object_filters::*;

#[path = "choices/type_phrases.rs"]
mod type_phrases;
pub(crate) use type_phrases::*;

#[path = "choices/sequence_shapes.rs"]
mod sequence_shapes;
pub(crate) use sequence_shapes::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceClauseActor {
    Implicit,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceClauseSeparator {
    And,
    Become,
    Then,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceClauseSeparatorSpan {
    pub(crate) first: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceObjectCountSource {
    CardsDiscardedThisWay,
    ThatMany,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChoiceObjectReferenceFacts {
    pub(crate) references_it: bool,
    pub(crate) references_container_it: bool,
    pub(crate) explicit_container_reference: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceObjectClauseShape {
    pub(crate) actor: ChoiceClauseActor,
    pub(crate) filter_words: Vec<String>,
    pub(crate) count: ChoiceCount,
    pub(crate) count_source: Option<ChoiceObjectCountSource>,
    pub(crate) references: ChoiceObjectReferenceFacts,
    pub(crate) filter_facts: ChoiceObjectFilterFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChoiceObjectClauseKind {
    Object(ChoiceObjectClauseShape),
    CardName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceObjectClauseSyntaxError {
    MissingObject,
    MissingFilter,
    UnsupportedFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChoicePlayerClauseShape {
    pub(crate) filter: PlayerFilter,
    pub(crate) random: bool,
    pub(crate) exclude_previous_choices: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoicePlayerClauseSyntaxError {
    UnsupportedFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceCardTypeRevealShape {
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChoiceWordSpan {
    first: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerReferenceSuffix {
    FromIt,
    FromThem,
    InIt,
    InThem,
    FromThereIn,
}

pub(crate) fn parse_choice_clause_separator_tokens(
    tokens: &[OwnedLexToken],
    separator: ChoiceClauseSeparator,
) -> Option<ChoiceClauseSeparatorSpan> {
    let mut input = LexStream::new(tokens);
    let skipped = repeat_till(
        0..,
        any.void(),
        peek(choice_separator_lexed(separator)).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    let first = skipped.len();
    choice_separator_lexed(separator)
        .parse_next(&mut input)
        .ok()?;
    Some(ChoiceClauseSeparatorSpan {
        first,
        end: tokens.len().checked_sub(input.len())?,
    })
}

pub(crate) fn parse_choice_object_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<ChoiceObjectClauseKind>, ChoiceObjectClauseSyntaxError> {
    let mut input = LexStream::new(tokens);
    let actor = match parse_choice_head_lexed.parse_next(&mut input) {
        Ok(actor) => actor,
        Err(_) => return Ok(None),
    };
    let consumed = tokens.len().saturating_sub(input.len());
    let body_tokens = trim_comma_edges(tokens.get(consumed..).unwrap_or_default());
    if body_tokens.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingObject);
    }

    let mut words = TokenWordView::new(body_tokens)
        .to_word_refs()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut count_source = strip_discarded_this_way_count_suffix(&mut words);
    let mut references = ChoiceObjectReferenceFacts::default();
    while let Some(suffix) = parse_container_reference_suffix(&words) {
        let removed = match suffix {
            ContainerReferenceSuffix::FromThereIn => 3,
            ContainerReferenceSuffix::FromIt
            | ContainerReferenceSuffix::FromThem
            | ContainerReferenceSuffix::InIt
            | ContainerReferenceSuffix::InThem => 2,
        };
        words.truncate(words.len().saturating_sub(removed));
        references.references_it = true;
        references.references_container_it = true;
        references.explicit_container_reference = true;
    }

    let refs = string_word_refs(&words);
    let mut count = ChoiceCount::exactly(1);
    if phrase_is_prefix(&refs, &["up", "to", "that", "many"]) {
        count = ChoiceCount::up_to_dynamic_x();
        count_source = Some(ChoiceObjectCountSource::ThatMany);
        words.drain(..4);
    } else if phrase_is_prefix(&refs, &["that", "many"]) {
        count = ChoiceCount::dynamic_x();
        count_source = Some(ChoiceObjectCountSource::ThatMany);
        words.drain(..2);
    } else if let Some(parsed) = leaf::parse_leaf_choice_count_prefix_words(&refs) {
        count = parsed.count;
        words.drain(..parsed.consumed);
    } else if parse_leading_article(&refs) {
        words.drain(..1);
    }
    while let Some(span) = parse_random_modifier_span(&string_word_refs(&words)) {
        count = count.at_random();
        words.drain(span.first..span.end);
    }
    if parse_aura_eligibility_suffix(&words) {
        words.truncate(words.len().saturating_sub(4));
    }
    if matches!(
        count_source,
        Some(ChoiceObjectCountSource::CardsDiscardedThisWay)
    ) {
        count = ChoiceCount::dynamic_x();
    }
    if words.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingFilter);
    }
    if parse_card_name_suffix(&words) {
        return Ok(Some(ChoiceObjectClauseKind::CardName));
    }

    if parse_tagged_choice_whole(&words) {
        references.references_it = true;
        words = vec!["card".to_string()];
    } else if words.len() > 2 && parse_tagged_reference_prefix(&words) {
        references.references_it = true;
        if parse_tagged_cards_whole(&words) {
            references.references_container_it = true;
        }
        words.drain(..2);
    }
    while let Some(span) = parse_embedded_container_reference_span(&string_word_refs(&words)) {
        references.references_it = true;
        references.references_container_it = true;
        references.explicit_container_reference = true;
        words.drain(span.first..span.end);
    }

    let filter_facts = parse_choice_object_filter_facts_words(&string_word_refs(&words));
    Ok(Some(ChoiceObjectClauseKind::Object(
        ChoiceObjectClauseShape {
            actor,
            filter_words: words,
            count,
            count_source,
            references,
            filter_facts,
        },
    )))
}

pub(crate) fn parse_choice_player_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<ChoicePlayerClauseShape>, ChoicePlayerClauseSyntaxError> {
    let mut input = LexStream::new(tokens);
    if parse_choice_head_lexed.parse_next(&mut input).is_err() {
        return Ok(None);
    }

    let exclude_previous_choices = parse_choice_player_ordinal_prefix_lexed(&mut input);
    let base = match parse_choice_player_base_lexed.parse_next(&mut input) {
        Ok(base) => base,
        Err(_) => return Ok(None),
    };
    let random = opt(primitives::phrase(&["at", "random"]))
        .parse_next(&mut input)
        .is_ok_and(|value| value.is_some());
    let filter = match base {
        ChoicePlayerBase::Opponent => {
            if !input.is_empty() {
                return Err(ChoicePlayerClauseSyntaxError::UnsupportedFilter);
            }
            PlayerFilter::Opponent
        }
        ChoicePlayerBase::Player => parse_choice_player_filter_tail_lexed
            .parse_next(&mut input)
            .map_err(|_| ChoicePlayerClauseSyntaxError::UnsupportedFilter)?,
    };

    Ok(Some(ChoicePlayerClauseShape {
        filter,
        random,
        exclude_previous_choices,
    }))
}

pub(crate) fn parse_choice_card_type_reveal_shape_words(
    first: &[&str],
    second: &[&str],
) -> Option<ChoiceCardTypeRevealShape> {
    let mut first_input: primitives::WordSliceInput<'_> = first;
    repeat_till(
        0..,
        any.void(),
        peek(alt((
            primitives::word_slice_exact("choose"),
            primitives::word_slice_exact("chooses"),
        )))
        .void(),
    )
    .map(|((), ())| ())
    .parse_next(&mut first_input)
    .ok()?;
    alt((
        primitives::word_slice_exact("choose"),
        primitives::word_slice_exact("chooses"),
    ))
    .parse_next(&mut first_input)
    .ok()?;
    opt(alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
        primitives::word_slice_exact("the"),
    )))
    .parse_next(&mut first_input)
    .ok()?;
    word_phrase(&["card", "type"])
        .parse_next(&mut first_input)
        .ok()?;
    word_phrase(&["then", "reveal", "the", "top"])
        .parse_next(&mut first_input)
        .ok()?;
    let parsed_count = leaf::parse_leaf_number_prefix_words(first_input)?.into_fixed()?;
    first_input = first_input.get(parsed_count.1..)?;
    alt((
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("cards"),
    ))
    .parse_next(&mut first_input)
    .ok()?;
    word_phrase_at_end(first_input, &["of", "your", "library"])?;

    let mut second_input: primitives::WordSliceInput<'_> = second;
    alt((
        primitives::word_slice_exact("put"),
        primitives::word_slice_exact("puts"),
    ))
    .parse_next(&mut second_input)
    .ok()?;
    word_phrase_occurs(second, &["chosen", "type"])?;
    word_phrase_occurs(second, &["revealed", "this", "way"])?;
    word_phrase_occurs(second, &["into", "your", "hand"])?;
    word_phrase_occurs(second, &["bottom", "of", "your", "library"])?;

    Some(ChoiceCardTypeRevealShape {
        count: parsed_count.0,
    })
}

fn parse_choice_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ChoiceClauseActor> {
    let actor = opt(primitives::kw("you"))
        .map(|you| {
            if you.is_some() {
                ChoiceClauseActor::You
            } else {
                ChoiceClauseActor::Implicit
            }
        })
        .parse_next(input)?;
    alt((primitives::kw("choose"), primitives::kw("chooses"))).parse_next(input)?;
    Ok(actor)
}

fn choice_separator_lexed<'a>(
    separator: ChoiceClauseSeparator,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| match separator {
        ChoiceClauseSeparator::And => primitives::kw("and").void().parse_next(input),
        ChoiceClauseSeparator::Become => alt((primitives::kw("become"), primitives::kw("becomes")))
            .void()
            .parse_next(input),
        ChoiceClauseSeparator::Then => primitives::kw("then").void().parse_next(input),
    }
}

fn trim_comma_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        tokens = &tokens[..tokens.len().saturating_sub(1)];
    }
    tokens
}

fn string_word_refs(words: &[String]) -> Vec<&str> {
    words.iter().map(String::as_str).collect()
}

fn strip_discarded_this_way_count_suffix(
    words: &mut Vec<String>,
) -> Option<ChoiceObjectCountSource> {
    let refs = string_word_refs(words);
    let tail = refs.get(refs.len().checked_sub(6)?..)?;
    let mut input: primitives::WordSliceInput<'_> = tail;
    (
        primitives::word_slice_exact("for"),
        primitives::word_slice_exact("each"),
        alt((
            primitives::word_slice_exact("card"),
            primitives::word_slice_exact("cards"),
        )),
        primitives::word_slice_exact("discarded"),
        primitives::word_slice_exact("this"),
        primitives::word_slice_exact("way"),
        primitives::word_slice_eof,
    )
        .parse_next(&mut input)
        .ok()?;
    words.truncate(words.len().saturating_sub(6));
    Some(ChoiceObjectCountSource::CardsDiscardedThisWay)
}

fn parse_container_reference_suffix(words: &[String]) -> Option<ContainerReferenceSuffix> {
    let refs = string_word_refs(words);
    if let Some(tail) = refs.get(refs.len().checked_sub(3)?..)
        && phrase_is_whole(tail, &["from", "there", "in"])
    {
        return Some(ContainerReferenceSuffix::FromThereIn);
    }
    let tail = refs.get(refs.len().checked_sub(2)?..)?;
    for (phrase, suffix) in [
        (&["from", "it"][..], ContainerReferenceSuffix::FromIt),
        (&["from", "them"][..], ContainerReferenceSuffix::FromThem),
        (&["in", "it"][..], ContainerReferenceSuffix::InIt),
        (&["in", "them"][..], ContainerReferenceSuffix::InThem),
    ] {
        if phrase_is_whole(tail, phrase) {
            return Some(suffix);
        }
    }
    None
}

fn parse_leading_article(words: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
        primitives::word_slice_exact("the"),
    ))
    .parse_next(&mut input)
    .is_ok()
}

fn parse_random_modifier_span(words: &[&str]) -> Option<ChoiceWordSpan> {
    parse_specific_phrase_span(words, &["at", "random"])
}

fn parse_embedded_container_reference_span(words: &[&str]) -> Option<ChoiceWordSpan> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped = repeat_till(
        0..,
        any.void(),
        peek(alt((
            word_phrase(&["from", "it"]),
            word_phrase(&["from", "them"]),
            word_phrase(&["in", "it"]),
            word_phrase(&["in", "them"]),
        )))
        .void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    alt((
        word_phrase(&["from", "it"]),
        word_phrase(&["from", "them"]),
        word_phrase(&["in", "it"]),
        word_phrase(&["in", "them"]),
    ))
    .parse_next(&mut input)
    .ok()?;
    Some(ChoiceWordSpan {
        first: skipped.len(),
        end: words.len().checked_sub(input.len())?,
    })
}

fn parse_specific_phrase_span(
    words: &[&str],
    phrase: &'static [&'static str],
) -> Option<ChoiceWordSpan> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped = repeat_till(0.., any.void(), peek(word_phrase(phrase)).void())
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    word_phrase(phrase).parse_next(&mut input).ok()?;
    Some(ChoiceWordSpan {
        first: skipped.len(),
        end: words.len().checked_sub(input.len())?,
    })
}

fn parse_aura_eligibility_suffix(words: &[String]) -> bool {
    let refs = string_word_refs(words);
    let Some(tail) = refs.get(refs.len().checked_sub(4).unwrap_or(usize::MAX)..) else {
        return false;
    };
    [
        &["this", "aura", "can", "enchant"][..],
        &["this", "aura", "could", "enchant"][..],
        &["that", "aura", "can", "enchant"][..],
        &["that", "aura", "could", "enchant"][..],
    ]
    .into_iter()
    .any(|phrase| phrase_is_whole(tail, phrase))
}

fn parse_card_name_suffix(words: &[String]) -> bool {
    let refs = string_word_refs(words);
    refs.get(refs.len().checked_sub(2).unwrap_or(usize::MAX)..)
        .is_some_and(|tail| phrase_is_whole(tail, &["card", "name"]))
}

fn parse_tagged_choice_whole(words: &[String]) -> bool {
    let refs = string_word_refs(words);
    phrase_is_whole(&refs, &["of", "them"]) || phrase_is_whole(&refs, &["of", "those"])
}

fn parse_tagged_reference_prefix(words: &[String]) -> bool {
    let refs = string_word_refs(words);
    phrase_is_prefix(&refs, &["of", "them"]) || phrase_is_prefix(&refs, &["of", "those"])
}

fn parse_tagged_cards_whole(words: &[String]) -> bool {
    let refs = string_word_refs(words);
    phrase_is_whole(&refs, &["of", "those", "card"])
        || phrase_is_whole(&refs, &["of", "those", "cards"])
}

fn parse_choice_player_ordinal_prefix_lexed(input: &mut LexStream<'_>) -> usize {
    let mut excluded = 0usize;
    loop {
        let mut probe = input.clone();
        let parsed = alt((
            alt((
                primitives::kw("a"),
                primitives::kw("an"),
                primitives::kw("the"),
            ))
            .value(0usize),
            alt((primitives::kw("other"), primitives::kw("another"))).value(1usize),
            primitives::kw("second").value(1usize),
            primitives::kw("third").value(2usize),
        ))
        .parse_next(&mut probe);
        let Ok(parsed) = parsed else {
            break;
        };
        excluded = excluded.max(parsed);
        *input = probe;
    }
    excluded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoicePlayerBase {
    Player,
    Opponent,
}

fn parse_choice_player_base_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ChoicePlayerBase> {
    alt((
        primitives::kw("player").value(ChoicePlayerBase::Player),
        alt((primitives::kw("opponent"), primitives::kw("opponents")))
            .value(ChoicePlayerBase::Opponent),
    ))
    .parse_next(input)
}

fn parse_choice_player_filter_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        (
            primitives::kw("with"),
            opt(primitives::kw("the")),
            primitives::phrase(&["most", "life", "or", "tied", "for", "most", "life"]),
            eof,
        )
            .value(PlayerFilter::MostLifeTied),
        (
            alt((primitives::kw("who"), primitives::kw("that"))),
            primitives::kw("cast"),
            primitives::phrase(&["one", "or", "more"]),
            parse_card_type_lexed,
            alt((primitives::kw("spell"), primitives::kw("spells"))),
            primitives::phrase(&["this", "turn"]),
            eof,
        )
            .map(|(_, _, _, card_type, _, _, _)| PlayerFilter::CastCardTypeThisTurn(card_type)),
        eof.value(PlayerFilter::Any),
    ))
    .parse_next(input)
}

fn parse_card_type_lexed(input: &mut LexStream<'_>) -> WResult<CardType> {
    alt((
        primitives::kw("artifact").value(CardType::Artifact),
        primitives::kw("battle").value(CardType::Battle),
        primitives::kw("creature").value(CardType::Creature),
        primitives::kw("enchantment").value(CardType::Enchantment),
        primitives::kw("instant").value(CardType::Instant),
        primitives::kw("kindred").value(CardType::Kindred),
        primitives::kw("land").value(CardType::Land),
        primitives::kw("planeswalker").value(CardType::Planeswalker),
        primitives::kw("sorcery").value(CardType::Sorcery),
    ))
    .parse_next(input)
}

fn word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<primitives::WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut primitives::WordSliceInput<'a>| {
        for word in expected {
            primitives::word_slice_exact(word)
                .void()
                .parse_next(input)?;
        }
        Ok(())
    }
}

fn word_phrase_at_end(words: &[&str], expected: &'static [&'static str]) -> Option<()> {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till(0.., any.void(), peek((word_phrase(expected), eof)).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .ok()?;
    word_phrase(expected).parse_next(&mut input).ok()?;
    input.is_empty().then_some(())
}

fn word_phrase_occurs(words: &[&str], expected: &'static [&'static str]) -> Option<()> {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till(0.., any.void(), peek(word_phrase(expected)).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .ok()?;
    word_phrase(expected).parse_next(&mut input).ok()
}

fn phrase_is_whole(words: &[&str], expected: &'static [&'static str]) -> bool {
    primitives::parse_full_word_slice(words, word_phrase(expected)).is_some()
}

fn phrase_is_prefix(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    word_phrase(expected).parse_next(&mut input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn choice_object_shape_preserves_reference_and_dynamic_count_facts() {
        let tokens = lex_line(
            "You choose cards from it for each card discarded this way.",
            0,
        )
        .unwrap();
        let tokens = &tokens[..tokens.len() - 1];

        let ChoiceObjectClauseKind::Object(parsed) =
            parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
        else {
            panic!("expected object choice");
        };
        assert_eq!(parsed.actor, ChoiceClauseActor::You);
        assert_eq!(parsed.filter_words, ["cards"]);
        assert_eq!(parsed.count, ChoiceCount::dynamic_x());
        assert_eq!(
            parsed.count_source,
            Some(ChoiceObjectCountSource::CardsDiscardedThisWay)
        );
        assert!(parsed.references.references_it);
        assert!(parsed.references.references_container_it);
        assert!(parsed.references.explicit_container_reference);
    }

    #[test]
    fn choice_object_shape_preserves_up_to_prior_amount_count() {
        let tokens = lex_line("Choose up to that many target creatures you control.", 0).unwrap();
        let tokens = &tokens[..tokens.len() - 1];

        let ChoiceObjectClauseKind::Object(parsed) =
            parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
        else {
            panic!("expected object choice");
        };
        assert!(parsed.count.is_up_to_dynamic_x());
        assert_eq!(parsed.count_source, Some(ChoiceObjectCountSource::ThatMany));
        assert_eq!(
            parsed.filter_words,
            ["target", "creatures", "you", "control"]
        );
    }

    #[test]
    fn choice_player_shape_is_typed() {
        let tokens = lex_line(
            "Choose another player who cast one or more sorcery spells this turn",
            0,
        )
        .unwrap();
        let parsed = parse_choice_player_clause_tokens(&tokens).unwrap().unwrap();

        assert_eq!(
            parsed.filter,
            PlayerFilter::CastCardTypeThisTurn(CardType::Sorcery)
        );
        assert_eq!(parsed.exclude_previous_choices, 1);
        assert!(!parsed.random);
    }

    #[test]
    fn card_type_reveal_pair_returns_count() {
        let first = [
            "choose", "a", "card", "type", "then", "reveal", "the", "top", "four", "cards", "of",
            "your", "library",
        ];
        let second = [
            "put", "all", "cards", "of", "the", "chosen", "type", "revealed", "this", "way",
            "into", "your", "hand", "and", "the", "rest", "on", "the", "bottom", "of", "your",
            "library",
        ];

        assert_eq!(
            parse_choice_card_type_reveal_shape_words(&first, &second),
            Some(ChoiceCardTypeRevealShape { count: 4 })
        );
    }

    #[test]
    fn choice_separator_returns_typed_token_span() {
        let tokens = lex_line("those creatures become that type", 0).unwrap();
        let parsed =
            parse_choice_clause_separator_tokens(&tokens, ChoiceClauseSeparator::Become).unwrap();

        assert_eq!(parsed.first, 2);
        assert_eq!(parsed.end, 3);
    }
}
