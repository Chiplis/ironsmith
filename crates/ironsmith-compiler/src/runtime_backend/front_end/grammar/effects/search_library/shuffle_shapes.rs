use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::grammar::primitives;
use crate::runtime_backend::lexer::{
    LexStream, OwnedLexToken, parser_token_word_refs, trim_lexed_commas,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SearchShuffleGraveyardShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) optional_shuffle: bool,
    pub(crate) each_player_subject: bool,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
    pub(crate) owner_library_destination: bool,
    pub(crate) has_target_selector: bool,
    pub(crate) has_source_and_graveyard_clause: bool,
    pub(crate) has_hand_clause: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchShuffleObjectReference {
    General,
    SingularBackReference,
    PluralTaggedReference,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchShuffleObjectShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) owner_subject_target_tokens: Option<Vec<OwnedLexToken>>,
    pub(crate) possessive_owner_subject: bool,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
    pub(crate) reference: SearchShuffleObjectReference,
    pub(crate) owner_library_destination: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EachChosenPlayerSearchPutTopShape;

fn each_chosen_player_search_put_top<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["each", "of", "them"]).parse_next(input)?;
    alt((primitives::kw("searches"), primitives::kw("search"))).parse_next(input)?;
    primitives::phrase(&["their", "library", "for"]).parse_next(input)?;
    alt((primitives::kw("a"), primitives::kw("one"))).parse_next(input)?;
    primitives::kw("card").parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    alt((primitives::kw("shuffles"), primitives::kw("shuffle"))).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("puts"), primitives::kw("put"))).parse_next(input)?;
    primitives::phrase(&["that", "card", "on", "top"]).parse_next(input)?;
    opt(primitives::sentence_end()).parse_next(input)?;
    eof.void().parse_next(input)
}

pub(crate) fn parse_each_chosen_player_search_put_top_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachChosenPlayerSearchPutTopShape> {
    primitives::parse_all(
        tokens,
        each_chosen_player_search_put_top,
        "each chosen player searches then puts on top",
    )
    .ok()
    .map(|()| EachChosenPlayerSearchPutTopShape)
}

fn sequence_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("then"), primitives::kw("and")))
        .void()
        .parse_next(input)
}

fn shuffle_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("shuffle"), primitives::kw("shuffles")))
        .void()
        .parse_next(input)
}

fn library_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("library"), primitives::kw("libraries")))
        .void()
        .parse_next(input)
}

fn owner_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("owner").void(),
        primitives::kw("owners").void(),
        primitives::kw("owner's").void(),
        primitives::kw("owner’s").void(),
        primitives::kw("owners'").void(),
        primitives::kw("owners’").void(),
        primitives::phrase(&["that", "player's"]).void(),
        primitives::phrase(&["that", "player’s"]).void(),
    ))
    .void()
    .parse_next(input)
}

fn source_and_graveyard_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "artifact", "and"]),
        primitives::phrase(&["this", "permanent", "and"]),
        primitives::phrase(&["this", "card", "and"]),
    ))
    .void()
    .parse_next(input)
}

fn owner_of_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["the", "owner", "of"])
        .void()
        .parse_next(input)
}

fn possessive_owner_target_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let (owner, target_tokens) = tokens.split_last()?;
    if owner.parser_text() != "owner" || target_tokens.is_empty() {
        return None;
    }
    let normalized =
        crate::runtime_backend::grammar::activation_restrictions::
            parse_activation_possessive_owner_tokens(target_tokens);
    (normalized != target_tokens).then_some(normalized)
}

fn singular_back_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["it"]),
        primitives::phrase(&["them"]),
        primitives::phrase(&["that"]),
        primitives::phrase(&["that", "object"]),
        primitives::phrase(&["that", "card"]),
    ))
    .void()
    .parse_next(input)
}

fn plural_tagged_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["them"]),
        primitives::phrase(&["those"]),
        primitives::phrase(&["those", "cards"]),
        primitives::phrase(&["those", "objects"]),
    ))
    .void()
    .parse_next(input)
}

fn word_present(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(expected)).is_some()
}

fn complete_shape<'a>(
    tokens: &'a [OwnedLexToken],
    parser: fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    primitives::parse_prefix(tokens, parser)
        .is_some_and(|(_, rest)| parser_token_word_refs(rest).is_empty())
}

fn clause_body(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let trimmed = trim_lexed_commas(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(trimmed, sequence_intro) {
        trim_lexed_commas(rest)
    } else {
        trimmed
    }
}

pub(crate) fn parse_shuffle_graveyard_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchShuffleGraveyardShape<'_>> {
    let clause = clause_body(tokens);
    if clause.is_empty() || !word_present(clause, "graveyard") || !word_present(clause, "library") {
        return None;
    }
    let (shuffle_idx, (), after_shuffle) = primitives::find_prefix(clause, || shuffle_marker)?;
    if shuffle_idx > 3 {
        return None;
    }

    let mut subject_tokens = trim_lexed_commas(&clause[..shuffle_idx]);
    let optional_shuffle = subject_tokens.last().is_some_and(|token| {
        complete_shape(std::slice::from_ref(token), |input| {
            primitives::kw("may").void().parse_next(input)
        })
    });
    if optional_shuffle {
        subject_tokens = &subject_tokens[..subject_tokens.len() - 1];
    }
    let each_player_subject =
        primitives::parse_prefix(subject_tokens, |input: &mut LexStream<'_>| {
            alt((
                primitives::phrase(&["each", "player"]),
                primitives::phrase(&["each", "players"]),
            ))
            .void()
            .parse_next(input)
        })
        .is_some();

    let body = trim_lexed_commas(after_shuffle);
    let (into_idx, (), after_into) =
        primitives::find_prefix(body, || primitives::kw("into").void())?;
    if into_idx == 0 {
        return None;
    }
    let target_tokens = trim_lexed_commas(&body[..into_idx]);
    if target_tokens.is_empty() || !word_present(target_tokens, "graveyard") {
        return None;
    }
    let destination = trim_lexed_commas(after_into);
    let (_, (), after_library) = primitives::find_prefix(destination, || library_marker)?;

    Some(SearchShuffleGraveyardShape {
        subject_tokens,
        optional_shuffle,
        each_player_subject,
        target_tokens,
        trailing_tokens: trim_lexed_commas(after_library),
        owner_library_destination: primitives::find_prefix(destination, || owner_marker).is_some(),
        has_target_selector: word_present(target_tokens, "target"),
        has_source_and_graveyard_clause: primitives::parse_prefix(
            target_tokens,
            source_and_graveyard_prefix,
        )
        .is_some(),
        has_hand_clause: word_present(target_tokens, "hand"),
    })
}

pub(crate) fn parse_shuffle_object_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchShuffleObjectShape<'_>> {
    let clause = clause_body(tokens);
    if clause.is_empty() || !word_present(clause, "library") || word_present(clause, "graveyard") {
        return None;
    }
    let (shuffle_idx, (), after_shuffle) = primitives::find_prefix(clause, || shuffle_marker)?;
    let subject_tokens = trim_lexed_commas(&clause[..shuffle_idx]);
    let owner_of_subject_target = primitives::parse_prefix(subject_tokens, owner_of_prefix)
        .map(|(_, rest)| trim_lexed_commas(rest).to_vec());
    let possessive_owner_subject = owner_of_subject_target.is_none();
    let owner_subject_target_tokens =
        owner_of_subject_target.or_else(|| possessive_owner_target_tokens(subject_tokens));
    if shuffle_idx > 3 && owner_subject_target_tokens.is_none() {
        return None;
    }

    let body = trim_lexed_commas(after_shuffle);
    let (into_idx, (), after_into) =
        primitives::find_prefix(body, || primitives::kw("into").void())?;
    if into_idx == 0 {
        return None;
    }
    let target_tokens = trim_lexed_commas(&body[..into_idx]);
    if target_tokens.is_empty() {
        return None;
    }
    let destination = trim_lexed_commas(after_into);
    let (library_idx, (), after_library) = primitives::find_prefix(destination, || library_marker)?;
    let reference = if owner_subject_target_tokens.is_some()
        && complete_shape(target_tokens, singular_back_reference)
    {
        SearchShuffleObjectReference::SingularBackReference
    } else if complete_shape(target_tokens, plural_tagged_reference) {
        SearchShuffleObjectReference::PluralTaggedReference
    } else {
        SearchShuffleObjectReference::General
    };

    Some(SearchShuffleObjectShape {
        subject_tokens,
        owner_subject_target_tokens,
        possessive_owner_subject,
        target_tokens,
        trailing_tokens: trim_lexed_commas(after_library),
        reference,
        owner_library_destination: primitives::find_prefix(destination.get(..library_idx)?, || {
            owner_marker
        })
        .is_some(),
    })
}

#[cfg(test)]
#[path = "shuffle_shapes/tests.rs"]
mod tests;
