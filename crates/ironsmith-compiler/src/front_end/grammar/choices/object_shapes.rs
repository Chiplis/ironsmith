use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::ChoiceCount;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use super::super::{leaf, primitives};
use super::{ChoiceObjectClauseSyntaxError, word_phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetPlayerChoiceActor {
    TargetPlayer,
    TargetOpponent,
    Opponent,
    ThatPlayer,
    Voter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PossessiveObjectChoiceActor {
    You,
    SubjectPlayer,
    ObjectController,
    Opponent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PossessiveObjectChoiceShape {
    pub(crate) actor: PossessiveObjectChoiceActor,
    pub(crate) object_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChoiceObjectFilterFacts {
    pub(crate) bare_card: bool,
    pub(crate) graveyard_and_hand: bool,
    pub(crate) tagged_graveyard_disjunction: bool,
    pub(crate) graveyard_arm_is_plain_card: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TargetPlayerChoiceShape<'a> {
    pub(crate) actor: TargetPlayerChoiceActor,
    pub(crate) count: ChoiceCount,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    /// The same clause beginning at its `choose`/`chooses` verb. This lets the
    /// shared object-choice grammar retain trailing dynamic counts and chosen-
    /// set constraints without duplicating those rules for player subjects.
    pub(crate) object_choice_tokens: &'a [OwnedLexToken],
    pub(crate) filter_facts: ChoiceObjectFilterFacts,
    pub(crate) filter_is_player_target: bool,
}

pub(crate) fn parse_target_player_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetPlayerChoiceShape<'_>>, ChoiceObjectClauseSyntaxError> {
    let mut input = LexStream::new(tokens);
    let actor = match parse_target_player_choice_head.parse_next(&mut input) {
        Ok(actor) => actor,
        Err(_) => return Ok(None),
    };
    let consumed = tokens.len().saturating_sub(input.len());
    let object_choice_tokens = tokens.get(consumed.saturating_sub(1)..).unwrap_or_default();
    let body = trim_punctuation_edges(tokens.get(consumed..).unwrap_or_default());
    if body.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingObject);
    }

    let (count, filter_tokens) =
        if let Some(parsed) = leaf::parse_leaf_choice_count_prefix_tokens(body) {
            (
                parsed.count,
                trim_punctuation_edges(body.get(parsed.consumed..).unwrap_or_default()),
            )
        } else {
            (ChoiceCount::exactly(1), body)
        };
    if filter_tokens.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingFilter);
    }

    let filter_words = TokenWordView::new(filter_tokens).word_refs();
    Ok(Some(TargetPlayerChoiceShape {
        actor,
        count,
        filter_tokens,
        object_choice_tokens,
        filter_facts: parse_choice_object_filter_facts_words(&filter_words),
        filter_is_player_target: parse_player_target_prefix_words(&filter_words),
    }))
}

/// Remove an embedded choice-owner phrase while retaining the complete object
/// and zone description around it, e.g. `a creature card of their choice from
/// their graveyard` -> `a creature card from their graveyard`.
pub(crate) fn parse_possessive_object_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PossessiveObjectChoiceShape> {
    for (phrase, actor) in [
        (
            &["of", "its", "controller's", "choice"][..],
            PossessiveObjectChoiceActor::ObjectController,
        ),
        (
            &["of", "its", "controllers", "choice"][..],
            PossessiveObjectChoiceActor::ObjectController,
        ),
        (
            &["of", "his", "or", "her", "choice"][..],
            PossessiveObjectChoiceActor::SubjectPlayer,
        ),
        (
            &["of", "their", "choice"][..],
            PossessiveObjectChoiceActor::SubjectPlayer,
        ),
        (
            &["of", "your", "choice"][..],
            PossessiveObjectChoiceActor::You,
        ),
        (
            &["of", "an", "opponent's", "choice"][..],
            PossessiveObjectChoiceActor::Opponent,
        ),
        (
            &["of", "an", "opponents", "choice"][..],
            PossessiveObjectChoiceActor::Opponent,
        ),
    ] {
        let Some((first, _, rest)) =
            primitives::find_prefix(tokens, || primitives::phrase(phrase).void())
        else {
            continue;
        };
        let mut object_tokens = Vec::with_capacity(first + rest.len());
        object_tokens.extend_from_slice(tokens.get(..first)?);
        object_tokens.extend_from_slice(rest);
        let object_tokens = trim_punctuation_edges(&object_tokens).to_vec();
        if !object_tokens.is_empty() {
            return Some(PossessiveObjectChoiceShape {
                actor,
                object_tokens,
            });
        }
    }
    None
}

pub(crate) fn parse_choice_object_filter_facts_words(words: &[&str]) -> ChoiceObjectFilterFacts {
    let has_graveyard = word_occurs(words, parse_graveyard_word);
    let has_hand = word_occurs(words, parse_hand_word);
    let has_or = word_occurs(words, primitives::word_slice_exact("or").void());
    ChoiceObjectFilterFacts {
        bare_card: primitives::parse_full_word_slice(words, parse_card_word).is_some(),
        graveyard_and_hand: has_graveyard && has_hand,
        tagged_graveyard_disjunction: has_graveyard && has_or,
        graveyard_arm_is_plain_card: phrase_occurs(words, &["or", "a", "card", "from"])
            || phrase_occurs(words, &["or", "the", "card", "from"])
            || phrase_occurs(words, &["or", "card", "from"]),
    }
}

fn parse_target_player_choice_head(input: &mut LexStream<'_>) -> WResult<TargetPlayerChoiceActor> {
    let actor = alt((
        (primitives::kw("target"), primitives::kw("player"))
            .value(TargetPlayerChoiceActor::TargetPlayer),
        (
            primitives::kw("target"),
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
        )
            .value(TargetPlayerChoiceActor::TargetOpponent),
        (opt(primitives::kw("an")), primitives::kw("opponent"))
            .value(TargetPlayerChoiceActor::Opponent),
        (
            primitives::kw("that"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(TargetPlayerChoiceActor::ThatPlayer),
        (primitives::kw("the"), primitives::kw("voter")).value(TargetPlayerChoiceActor::Voter),
    ))
    .parse_next(input)?;
    alt((primitives::kw("choose"), primitives::kw("chooses"))).parse_next(input)?;
    Ok(actor)
}

fn parse_player_target_prefix_words(words: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        primitives::word_slice_exact("target"),
        alt((
            primitives::word_slice_exact("player"),
            primitives::word_slice_exact("opponent"),
        )),
    )
        .parse_next(&mut input)
        .is_ok()
}

fn parse_card_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("cards"),
    ))
    .void()
    .parse_next(input)
}

fn parse_graveyard_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("graveyard"),
        primitives::word_slice_exact("graveyards"),
    ))
    .void()
    .parse_next(input)
}

fn parse_hand_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("hand"),
        primitives::word_slice_exact("hands"),
    ))
    .void()
    .parse_next(input)
}

fn word_occurs<'a, P>(words: &'a [&'a str], parser: P) -> bool
where
    P: Parser<primitives::WordSliceInput<'a>, (), ErrMode<ContextError>>,
{
    let mut input: primitives::WordSliceInput<'a> = words;
    repeat_till(0.., any.void(), peek(parser).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .is_ok()
}

fn phrase_occurs(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till(0.., any.void(), peek(word_phrase(expected)).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .is_ok()
}

fn trim_punctuation_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[..tokens.len().saturating_sub(1)];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn target_player_choice_head_returns_typed_actor_count_and_filter() {
        let tokens = lex("Target opponent chooses up to two creatures from a graveyard or hand.");
        let parsed = parse_target_player_choice_tokens(&tokens).unwrap().unwrap();

        assert_eq!(parsed.actor, TargetPlayerChoiceActor::TargetOpponent);
        assert_eq!(parsed.count, ChoiceCount::up_to(2));
        assert_eq!(
            TokenWordView::new(parsed.filter_tokens).word_refs()[0],
            "creatures"
        );
        assert!(parsed.filter_facts.graveyard_and_hand);

        let opponent = lex("An opponent chooses up to two nonland permanents they control.");
        let opponent = parse_target_player_choice_tokens(&opponent)
            .unwrap()
            .expect("indefinite opponent choice should use the same typed grammar");
        assert_eq!(opponent.actor, TargetPlayerChoiceActor::Opponent);
        assert_eq!(opponent.count, ChoiceCount::up_to(2));

        let article_tokens = lex("Target opponent chooses a card.");
        let article = parse_target_player_choice_tokens(&article_tokens)
            .unwrap()
            .unwrap();
        assert_eq!(article.count, ChoiceCount::exactly(1));
        assert_eq!(
            TokenWordView::new(article.filter_tokens).word_refs(),
            vec!["card"]
        );
    }

    #[test]
    fn object_filter_facts_cover_tagged_disjunction_and_bare_card() {
        let facts = parse_choice_object_filter_facts_words(&[
            "card",
            "from",
            "it",
            "or",
            "a",
            "card",
            "from",
            "a",
            "graveyard",
        ]);
        assert!(facts.tagged_graveyard_disjunction);
        assert!(facts.graveyard_arm_is_plain_card);
        assert!(!facts.bare_card);

        assert!(parse_choice_object_filter_facts_words(&["cards"]).bare_card);
    }

    #[test]
    fn possessive_choice_keeps_zone_tail_and_choice_owner() {
        let tokens = lex("a creature card of their choice from their graveyard");
        let parsed = parse_possessive_object_choice_tokens(&tokens).unwrap();

        assert_eq!(parsed.actor, PossessiveObjectChoiceActor::SubjectPlayer);
        assert_eq!(
            TokenWordView::new(&parsed.object_tokens).word_refs(),
            vec!["a", "creature", "card", "from", "their", "graveyard"]
        );
    }
}
