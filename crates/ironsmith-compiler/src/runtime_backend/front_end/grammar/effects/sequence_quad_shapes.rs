use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::runtime_backend::grammar::{leaf, primitives};
use crate::runtime_backend::lexer::{LexStream, LexedClause, OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy)]
pub(crate) struct NamedRevealedCardShape<'a> {
    pub(crate) name_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CountedLookedCardExileShape {
    pub(crate) count: ChoiceCount,
    pub(crate) includes_remainder: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExiledCardCastFilterShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LookedCardFilterShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct LookedCardRevealShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) count: ChoiceCount,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LookExileSplitShape<'a> {
    pub(crate) look_tokens: &'a [OwnedLexToken],
    pub(crate) exile_tokens: &'a [OwnedLexToken],
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("the"),
        primitives::kw("a"),
        primitives::kw("an"),
    ))
    .void()
    .parse_next(input)
}

fn exact_unit<'a>(
    tokens: &'a [OwnedLexToken],
    parser: fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    primitives::parse_prefix(trimmed(tokens), parser)
        .is_some_and(|(_, rest)| trimmed(rest).is_empty())
}

fn is_article_token(token: &OwnedLexToken) -> bool {
    exact_unit(std::slice::from_ref(token), article)
}

fn without_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !is_article_token(token))
        .cloned()
        .collect()
}

fn if_you_reveal<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "reveal"])
        .void()
        .parse_next(input)
}

fn this_way<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["this", "way"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_named_revealed_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<NamedRevealedCardShape<'_>> {
    let clause = trimmed(tokens);
    let ((), after_intro) = primitives::parse_prefix(clause, if_you_reveal)?;
    let (named_idx, (), after_named) =
        primitives::find_prefix(after_intro, || primitives::kw("named").void())?;
    let _ = named_idx;
    let (this_way_idx, (), _) = primitives::find_prefix(after_named, || this_way)?;
    let name_tokens = trimmed(&after_named[..this_way_idx]);
    (!name_tokens.is_empty()).then_some(NamedRevealedCardShape { name_tokens })
}

fn put_looked_onto_battlefield<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "it", "onto", "the", "battlefield"]),
        primitives::phrase(&["put", "that", "card", "onto", "the", "battlefield"]),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_put_looked_onto_battlefield_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(trimmed(tokens), || put_looked_onto_battlefield).is_some()
}

fn put_looked_into_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "that", "card", "into", "your", "hand"]),
        primitives::phrase(&["put", "it", "into", "your", "hand"]),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_put_looked_into_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    let mut clause = trimmed(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("otherwise").void().parse_next(input)
    }) {
        clause = trimmed(rest);
    }
    primitives::parse_prefix(clause, put_looked_into_hand).is_some()
}

fn then_shuffle<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["then", "shuffle"]),
        primitives::kw("shuffle").void(),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_then_shuffle_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, then_shuffle)
}

fn exile_one_face_down<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["exile", "one", "of", "them", "face", "down"])
        .void()
        .parse_next(input)
}

fn put_rest<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["put", "rest"])
        .void()
        .parse_next(input)
}

fn bottom_of_your_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["bottom", "of", "your", "library"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_exile_one_and_bottom_remainder_shape(tokens: &[OwnedLexToken]) -> bool {
    let normalized = without_articles(trimmed(tokens));
    primitives::parse_prefix(&normalized, exile_one_face_down).is_some()
        && primitives::find_prefix(&normalized, || put_rest).is_some()
        && primitives::find_prefix(&normalized, || bottom_of_your_library).is_some()
}

fn counted_exile_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["of", "them", "face", "down"]),
        primitives::phrase(&["of", "those", "cards", "face", "down"]),
        primitives::phrase(&["them", "face", "down"]),
        primitives::phrase(&["those", "cards", "face", "down"]),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_counted_looked_card_exile_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedLookedCardExileShape> {
    let clause = trimmed(tokens);
    let ((), count_surface) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("exile").void().parse_next(input)
    })?;
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(trimmed(count_surface))?;
    let tail = trimmed(&trimmed(count_surface)[parsed.consumed..]);
    let normalized_tail = without_articles(tail);
    primitives::parse_prefix(&normalized_tail, counted_exile_tail)?;
    Some(CountedLookedCardExileShape {
        count: parsed.count,
        includes_remainder: primitives::find_prefix(&normalized_tail, || put_rest).is_some(),
    })
}

fn put_remainder_on_bottom<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "rest", "on", "bottom"]),
        primitives::phrase(&["put", "rest", "onto", "bottom"]),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_looked_remainder_bottom_shape(
    tokens: &[OwnedLexToken],
) -> Option<LibraryBottomOrderAst> {
    let clause = trimmed(tokens);
    let normalized = without_articles(clause);
    primitives::find_prefix(&normalized, || put_remainder_on_bottom)?;
    primitives::find_prefix(&normalized, || primitives::kw("library").void())?;
    let words = TokenWordView::new(clause).word_refs();
    super::sequence_pairs::parse_consult_remainder_order_shape(&words)
}

fn cast_exiled_free<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "you", "may", "cast", "exiled", "card", "without", "paying", "its", "mana", "cost",
    ])
    .void()
    .parse_next(input)
}

fn exiled_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("its"),
        primitives::kw("it's"),
        primitives::kw("it"),
        primitives::kw("that"),
        primitives::kw("that's"),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_exiled_card_cast_filter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExiledCardCastFilterShape<'_>> {
    let clause = trimmed(tokens);
    let (if_idx, (), after_if) = primitives::find_prefix(clause, || primitives::kw("if").void())?;
    let prefix = without_articles(trimmed(&clause[..if_idx]));
    if !exact_unit(&prefix, cast_exiled_free) {
        return None;
    }
    let mut condition = trimmed(after_if);
    if let Some(((), rest)) = primitives::parse_prefix(condition, exiled_reference) {
        condition = trimmed(rest);
    }
    if let Some(((), rest)) = primitives::parse_prefix(condition, |input: &mut LexStream<'_>| {
        primitives::kw("card").void().parse_next(input)
    }) {
        condition = trimmed(rest);
    }
    (!condition.is_empty()).then_some(ExiledCardCastFilterShape {
        filter_tokens: condition,
    })
}

fn exiled_card_hand_followup<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you"]).parse_next(input)?;
    alt((
        primitives::kw("don't").void(),
        primitives::kw("dont").void(),
        primitives::phrase(&["do", "not"]).void(),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["put", "that", "card", "into", "your", "hand"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_exiled_card_hand_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, exiled_card_hand_followup)
}

fn may_reveal_looked<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["you", "may", "reveal"])
        .void()
        .parse_next(input)
}

fn from_among_them<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["from", "among", "them"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_may_reveal_looked_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardRevealShape<'_>> {
    let clause = trimmed(tokens);
    let ((), count_surface) = primitives::parse_prefix(clause, may_reveal_looked)?;
    let count_surface = trimmed(count_surface);
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(count_surface)?;
    let filter_surface = trimmed(&count_surface[parsed.consumed..]);
    let (among_idx, (), _) = primitives::find_prefix(filter_surface, || from_among_them)?;
    let filter_tokens = trimmed(&filter_surface[..among_idx]);
    (!filter_tokens.is_empty()).then_some(LookedCardRevealShape {
        filter_tokens,
        count: parsed.count,
    })
}

fn bargained<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "this", "spell", "was", "bargained"])
        .void()
        .parse_next(input)
}

fn put_revealed_onto_battlefield<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "put",
        "the",
        "revealed",
        "cards",
        "onto",
        "the",
        "battlefield",
    ])
    .void()
    .parse_next(input)
}

pub(crate) fn parse_bargained_revealed_battlefield_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    primitives::parse_prefix(clause, bargained).is_some()
        && primitives::find_prefix(clause, || put_revealed_onto_battlefield).is_some()
}

fn otherwise_revealed_into_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("otherwise").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["put", "the", "revealed", "cards", "into", "your", "hand"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_otherwise_revealed_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(trimmed(tokens), otherwise_revealed_into_hand).is_some()
}

fn may_exile_looked<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["you", "may", "exile"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_may_exile_looked_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardFilterShape<'_>> {
    let clause = trimmed(tokens);
    let ((), filter_surface) = primitives::parse_prefix(clause, may_exile_looked)?;
    let filter_surface = trimmed(filter_surface);
    let (among_idx, (), after_among) = primitives::find_prefix(filter_surface, || from_among_them)?;
    if !trimmed(after_among).is_empty() {
        return None;
    }
    let filter_tokens = trimmed(&filter_surface[..among_idx]);
    (!filter_tokens.is_empty()).then_some(LookedCardFilterShape { filter_tokens })
}

pub(crate) fn parse_look_exile_split_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookExileSplitShape<'_>> {
    let clause = trimmed(tokens);
    let (exile_idx, (), _) = primitives::find_prefix(clause, || primitives::kw("exile").void())?;
    Some(LookExileSplitShape {
        look_tokens: trimmed(&clause[..exile_idx]),
        exile_tokens: trimmed(&clause[exile_idx..]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_counted_exile_and_filter_shapes() {
        let counted = parse_counted_looked_card_exile_shape(&lex(
            "Exile up to two of them face down, then put the rest on the bottom",
        ))
        .unwrap();
        assert_eq!(counted.count, ChoiceCount::up_to(2));
        assert!(counted.includes_remainder);

        let reveal_tokens = lex("You may reveal up to one creature card from among them");
        let reveal = parse_may_reveal_looked_card_shape(&reveal_tokens).unwrap();
        assert_eq!(reveal.count, ChoiceCount::up_to(1));
        assert_eq!(
            TokenWordView::new(reveal.filter_tokens).word_refs(),
            vec!["creature", "card"]
        );
        assert!(parse_otherwise_revealed_hand_shape(&lex(
            "Otherwise, put the revealed cards into your hand"
        )));
    }

    #[test]
    fn parses_named_and_sequence_markers() {
        let named_tokens =
            lex("If you reveal a card named black lotus this way, put it onto the battlefield");
        let named = parse_named_revealed_card_shape(&named_tokens).unwrap();
        assert_eq!(
            TokenWordView::new(named.name_tokens).word_refs(),
            vec!["black", "lotus"]
        );
        assert!(parse_then_shuffle_shape(&lex("then shuffle")));
    }

    #[test]
    fn parses_discover_the_impossible_sequence_shapes() {
        assert!(parse_exile_one_and_bottom_remainder_shape(&lex(
            "Exile one of them face down and put the rest on the bottom of your library in a random order"
        )));

        let cast_tokens = lex(
            "You may cast the exiled card without paying its mana cost if it's an instant spell with mana value 2 or less",
        );
        let cast = parse_exiled_card_cast_filter_shape(&cast_tokens)
            .expect("free-cast condition should expose its typed filter surface");
        assert_eq!(
            TokenWordView::new(cast.filter_tokens).word_refs(),
            vec![
                "an", "instant", "spell", "with", "mana", "value", "2", "or", "less"
            ]
        );

        assert!(parse_exiled_card_hand_followup_shape(&lex(
            "If you don't, put that card into your hand"
        )));
    }
}
