use winnow::combinator::{peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivvyChooserShape {
    Opponent,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivvyRestDestinationShape {
    Hand,
    BattlefieldTapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivvySequenceShape {
    SearchFourCreatureCards,
    SearchLibraryGraveyardExileRemainderToTop,
    ExchangeCreatureControl,
    DestroyChosenCreaturePile,
    GraveyardCreaturePiles,
    OpponentCreaturePilesSacrifice,
    PermanentPilesSacrifice,
    DefendingCreaturePilesBlock,
    CreaturePilesAttack,
    LandPiles,
    ExilePermanentCardsPile,
    RevealTopPiles,
    ExileCreatureCardsFromGraveyards,
    ChooseOneOfThem,
    SearchFourDifferentNames {
        chooser: DivvyChooserShape,
        rest: DivvyRestDestinationShape,
    },
    TargetOpponentChoosesOne,
}

fn normalized_word_eq(actual: &str, expected: &str) -> bool {
    actual
        .chars()
        .filter(|ch| *ch != '\'')
        .eq(expected.chars().filter(|ch| *ch != '\''))
}

fn dynamic_phrase<'a>(
    expected: &'a [&'a str],
) -> impl Parser<primitives::WordSliceInput<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>
+ 'a {
    move |input: &mut primitives::WordSliceInput<'a>| {
        let mut rest = *input;
        for expected_word in expected {
            let Some((actual, tail)) = rest.split_first() else {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            };
            if !normalized_word_eq(actual, expected_word) {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            }
            rest = tail;
        }
        *input = rest;
        Ok(())
    }
}

fn exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (dynamic_phrase(expected), primitives::word_slice_eof)
        .parse_next(&mut input)
        .is_ok()
}

fn prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    dynamic_phrase(expected).parse_next(&mut input).is_ok()
}

fn phrase_anywhere(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(dynamic_phrase(expected)))
        .void()
        .parse_next(&mut input)
        .is_ok()
}

fn exact_sequence(sentence_words: &[Vec<&str>], expected: &[&[&str]]) -> bool {
    sentence_words.len() == expected.len()
        && sentence_words
            .iter()
            .zip(expected)
            .all(|(actual, expected)| exact(actual, expected))
}

fn sequence_has_phrase(sentence_words: &[Vec<&str>], phrase: &[&str]) -> bool {
    sentence_words
        .iter()
        .any(|words| phrase_anywhere(words, phrase))
}

pub fn parse_divvy_sequence_shape(sentences: &[&[OwnedLexToken]]) -> Option<DivvySequenceShape> {
    let sentence_words = sentences
        .iter()
        .map(|tokens| TokenWordView::new(tokens).to_word_refs())
        .collect::<Vec<_>>();
    let first = sentence_words.first().map(Vec::as_slice).unwrap_or(&[]);

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "search",
                "your",
                "library",
                "and",
                "graveyard",
                "for",
                "five",
                "cards",
                "and",
                "exile",
                "the",
                "rest",
            ],
            &[
                "put", "the", "chosen", "cards", "on", "top", "of", "your", "library", "in", "any",
                "order",
            ],
            &["you", "lose", "half", "your", "life", "rounded", "up"],
        ],
    ) {
        return Some(DivvySequenceShape::SearchLibraryGraveyardExileRemainderToTop);
    }

    if sentences.len() == 1
        && prefix(
            first,
            &[
                "search",
                "your",
                "library",
                "and",
                "graveyard",
                "for",
                "up",
                "to",
                "four",
                "creature",
                "cards",
            ],
        )
        && phrase_anywhere(first, &["chooses", "two", "of", "those", "cards"])
        && phrase_anywhere(first, &["shuffle", "the", "chosen", "cards"])
        && phrase_anywhere(first, &["put", "the", "rest", "onto", "the", "battlefield"])
    {
        return Some(DivvySequenceShape::SearchFourCreatureCards);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "choose",
                "any",
                "number",
                "of",
                "creatures",
                "target",
                "player",
                "controls",
            ],
            &[
                "choose",
                "the",
                "same",
                "number",
                "of",
                "creatures",
                "another",
                "target",
                "player",
                "controls",
            ],
            &[
                "those",
                "players",
                "exchange",
                "control",
                "of",
                "those",
                "creatures",
            ],
        ],
    ) {
        return Some(DivvySequenceShape::ExchangeCreatureControl);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "separate",
                "all",
                "creatures",
                "target",
                "player",
                "controls",
                "into",
                "two",
                "piles",
            ],
            &[
                "destroy",
                "all",
                "creatures",
                "in",
                "the",
                "pile",
                "of",
                "that",
                "player's",
                "choice",
            ],
            &["they", "can't", "be", "regenerated"],
        ],
    ) {
        return Some(DivvySequenceShape::DestroyChosenCreaturePile);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "separate",
                "all",
                "creature",
                "cards",
                "in",
                "your",
                "graveyard",
                "into",
                "two",
                "piles",
            ],
            &[
                "exile",
                "the",
                "pile",
                "of",
                "an",
                "opponent's",
                "choice",
                "and",
                "return",
                "the",
                "other",
                "to",
                "the",
                "battlefield",
            ],
        ],
    ) {
        return Some(DivvySequenceShape::GraveyardCreaturePiles);
    }

    if prefix(
        first,
        &[
            "each",
            "opponent",
            "separates",
            "the",
            "creatures",
            "they",
            "control",
            "into",
            "two",
            "piles",
        ],
    ) && sequence_has_phrase(&sentence_words, &["for", "each", "opponent"])
        && sequence_has_phrase(
            &sentence_words,
            &[
                "each",
                "opponent",
                "sacrifices",
                "the",
                "creatures",
                "in",
                "their",
                "chosen",
                "pile",
            ],
        )
    {
        return Some(DivvySequenceShape::OpponentCreaturePilesSacrifice);
    }

    if prefix(
        first,
        &[
            "separate",
            "all",
            "permanents",
            "target",
            "player",
            "controls",
            "into",
            "two",
            "piles",
        ],
    ) && sequence_has_phrase(
        &sentence_words,
        &[
            "that",
            "player",
            "sacrifices",
            "all",
            "permanents",
            "in",
            "the",
            "pile",
            "of",
            "their",
            "choice",
        ],
    ) {
        return Some(DivvySequenceShape::PermanentPilesSacrifice);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "for",
                "each",
                "defending",
                "player",
                "separate",
                "all",
                "creatures",
                "that",
                "player",
                "controls",
                "into",
                "two",
                "piles",
                "and",
                "that",
                "player",
                "chooses",
                "one",
            ],
            &[
                "only",
                "creatures",
                "in",
                "the",
                "chosen",
                "piles",
                "can",
                "block",
                "this",
                "turn",
            ],
        ],
    ) {
        return Some(DivvySequenceShape::DefendingCreaturePilesBlock);
    }

    if prefix(
        first,
        &[
            "separate",
            "all",
            "creatures",
            "that",
            "player",
            "controls",
            "into",
            "two",
            "piles",
        ],
    ) && sequence_has_phrase(
        &sentence_words,
        &[
            "only",
            "creatures",
            "in",
            "the",
            "pile",
            "of",
            "their",
            "choice",
            "can",
            "attack",
            "this",
            "turn",
        ],
    ) {
        return Some(DivvySequenceShape::CreaturePilesAttack);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "each",
                "player",
                "separates",
                "all",
                "nontoken",
                "lands",
                "they",
                "control",
                "into",
                "two",
                "piles",
            ],
            &[
                "for",
                "each",
                "player",
                "one",
                "of",
                "their",
                "piles",
                "is",
                "chosen",
                "by",
                "one",
                "of",
                "their",
                "opponents",
                "of",
                "their",
                "choice",
            ],
            &["destroy", "all", "lands", "in", "the", "chosen", "piles"],
            &["tap", "all", "lands", "in", "the", "other", "piles"],
        ],
    ) {
        return Some(DivvySequenceShape::LandPiles);
    }

    if prefix(
        first,
        &[
            "exile",
            "up",
            "to",
            "five",
            "target",
            "permanent",
            "cards",
            "from",
            "your",
            "graveyard",
            "and",
            "separate",
            "them",
            "into",
            "two",
            "piles",
        ],
    ) && sequence_has_phrase(
        &sentence_words,
        &["an", "opponent", "chooses", "one", "of", "those", "piles"],
    ) && sequence_has_phrase(
        &sentence_words,
        &["put", "that", "pile", "into", "your", "hand"],
    ) && sequence_has_phrase(
        &sentence_words,
        &["the", "other", "into", "your", "graveyard"],
    ) {
        return Some(DivvySequenceShape::ExilePermanentCardsPile);
    }

    if prefix(first, &["reveal", "the", "top"])
        && sequence_has_phrase(&sentence_words, &["cards", "of", "your", "library"])
        && sequence_has_phrase(
            &sentence_words,
            &[
                "an",
                "opponent",
                "separates",
                "those",
                "cards",
                "into",
                "two",
                "piles",
            ],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["put", "one", "pile", "into", "your", "hand"],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["the", "other", "into", "your", "graveyard"],
        )
    {
        return Some(DivvySequenceShape::RevealTopPiles);
    }

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "exile",
                "up",
                "to",
                "five",
                "target",
                "creature",
                "cards",
                "from",
                "graveyards",
            ],
            &[
                "an",
                "opponent",
                "separates",
                "those",
                "cards",
                "into",
                "two",
                "piles",
            ],
            &[
                "put",
                "all",
                "cards",
                "from",
                "the",
                "pile",
                "of",
                "your",
                "choice",
                "onto",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "and",
                "the",
                "rest",
                "into",
                "their",
                "owners'",
                "graveyards",
            ],
        ],
    ) {
        return Some(DivvySequenceShape::ExileCreatureCardsFromGraveyards);
    }

    if prefix(
        first,
        &[
            "search",
            "your",
            "library",
            "and",
            "graveyard",
            "for",
            "up",
            "to",
            "four",
            "creature",
            "cards",
        ],
    ) && sequence_has_phrase(&sentence_words, &["different", "names"])
        && sequence_has_phrase(&sentence_words, &["mana", "value", "x", "or", "less"])
        && sequence_has_phrase(&sentence_words, &["reveal", "them"])
        && sequence_has_phrase(
            &sentence_words,
            &["an", "opponent", "chooses", "two", "of", "those", "cards"],
        )
        && sequence_has_phrase(
            &sentence_words,
            &[
                "shuffle", "the", "chosen", "cards", "into", "your", "library",
            ],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["put", "the", "rest", "onto", "the", "battlefield"],
        )
    {
        return Some(DivvySequenceShape::SearchFourCreatureCards);
    }

    if sentences.len() >= 2
        && sequence_has_phrase(
            &sentence_words,
            &["an", "opponent", "chooses", "one", "of", "them"],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["put", "the", "chosen", "card", "into", "your", "hand"],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["the", "other", "into", "your", "graveyard"],
        )
    {
        return Some(DivvySequenceShape::ChooseOneOfThem);
    }

    if prefix(
        first,
        &["search", "your", "library", "for", "up", "to", "four"],
    ) && sequence_has_phrase(&sentence_words, &["cards", "with", "different", "names"])
        && sequence_has_phrase(&sentence_words, &["reveal", "them"])
        && sequence_has_phrase(
            &sentence_words,
            &["put", "the", "chosen", "cards", "into", "your", "graveyard"],
        )
        && sequence_has_phrase(&sentence_words, &["shuffle"])
    {
        let chooser = if sequence_has_phrase(
            &sentence_words,
            &[
                "target", "opponent", "chooses", "two", "of", "those", "cards",
            ],
        ) {
            DivvyChooserShape::TargetOpponent
        } else if sequence_has_phrase(
            &sentence_words,
            &["an", "opponent", "chooses", "two", "of", "those", "cards"],
        ) {
            DivvyChooserShape::Opponent
        } else {
            return None;
        };
        let rest = if sequence_has_phrase(&sentence_words, &["the", "rest", "into", "your", "hand"])
        {
            DivvyRestDestinationShape::Hand
        } else if sequence_has_phrase(
            &sentence_words,
            &["the", "rest", "onto", "the", "battlefield", "tapped"],
        ) {
            DivvyRestDestinationShape::BattlefieldTapped
        } else {
            return None;
        };
        return Some(DivvySequenceShape::SearchFourDifferentNames { chooser, rest });
    }

    if sequence_has_phrase(&sentence_words, &["target", "opponent", "chooses", "one"])
        && sequence_has_phrase(
            &sentence_words,
            &["put", "that", "card", "into", "your", "hand"],
        )
        && sequence_has_phrase(
            &sentence_words,
            &["the", "rest", "into", "your", "graveyard"],
        )
    {
        return Some(DivvySequenceShape::TargetOpponentChoosesOne);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn classifies_exchange_control_sequence() {
        let lines = [
            lex_line("Choose any number of creatures target player controls.", 0).unwrap(),
            lex_line(
                "Choose the same number of creatures another target player controls.",
                1,
            )
            .unwrap(),
            lex_line("Those players exchange control of those creatures.", 2).unwrap(),
        ];
        let slices = lines.iter().map(Vec::as_slice).collect::<Vec<_>>();
        assert_eq!(
            parse_divvy_sequence_shape(&slices),
            Some(DivvySequenceShape::ExchangeCreatureControl)
        );
    }

    #[test]
    fn classifies_multi_zone_search_exile_remainder_to_ordered_top() {
        let lines = [
            lex_line(
                "Search your library and graveyard for five cards and exile the rest.",
                0,
            )
            .unwrap(),
            lex_line(
                "Put the chosen cards on top of your library in any order.",
                1,
            )
            .unwrap(),
            lex_line("You lose half your life, rounded up.", 2).unwrap(),
        ];
        let slices = lines.iter().map(Vec::as_slice).collect::<Vec<_>>();
        assert_eq!(
            parse_divvy_sequence_shape(&slices),
            Some(DivvySequenceShape::SearchLibraryGraveyardExileRemainderToTop)
        );
    }
}
