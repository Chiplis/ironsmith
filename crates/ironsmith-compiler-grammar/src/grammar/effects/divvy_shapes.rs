use winnow::combinator::{peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenWordView};

#[path = "divvy_shapes/helpers.rs"]
mod helpers;
use helpers::*;

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
    FixedExilePiles { first_count: i32, second_count: i32, first_face_down: bool, second_face_down: bool },
    /// "Choose an opponent. They look at the top N cards of your library and
    /// separate them into a face-down pile and a face-up pile. Put one pile
    /// into your hand and the other into your graveyard." Any following
    /// sentences are independent.
    ChosenOpponentFaceDownPiles { count: i32, consumed_sentences: usize },
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
    SearchFourDifferentPowers,
    TargetOpponentChoosesOne,
}

pub fn parse_divvy_sequence_shape(sentences: &[&[OwnedLexToken]]) -> Option<DivvySequenceShape> {
    let sentence_words = sentences
        .iter()
        .map(|tokens| TokenWordView::new(tokens).to_word_refs())
        .collect::<Vec<_>>();
    let first = sentence_words.first().map(Vec::as_slice).unwrap_or(&[]);

    if let Some(shape) = parse_fixed_exile_piles(&sentence_words) {
        return Some(shape);
    }
    if let Some(shape) = parse_chosen_opponent_face_down_piles(&sentence_words) {
        return Some(shape);
    }

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

    if exact_sequence(
        &sentence_words,
        &[
            &[
                "search",
                "your",
                "library",
                "for",
                "up",
                "to",
                "four",
                "creature",
                "cards",
                "with",
                "different",
                "powers",
                "and",
                "reveal",
                "them",
            ],
            &["an", "opponent", "chooses", "two", "of", "those", "cards"],
            &[
                "shuffle", "the", "chosen", "cards", "into", "your", "library", "and", "put",
                "the", "rest", "into", "your", "hand",
            ],
        ],
    ) {
        return Some(DivvySequenceShape::SearchFourDifferentPowers);
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
#[path = "divvy_shapes_inline_tests.rs"]
mod tests;

fn parse_chosen_opponent_face_down_piles(sentences: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let [choose, look, put, ..] = sentences else {
        return None;
    };
    if choose.as_slice() != ["choose", "an", "opponent"]
        || put.as_slice()
            != [
                "put", "one", "pile", "into", "your", "hand", "and", "the", "other", "into",
                "your", "graveyard",
            ]
    {
        return None;
    }
    let words = look.strip_prefix(&["they", "look", "at", "the", "top"])?;
    let (count, used) =
        crate::grammar::leaf::parse_leaf_number_prefix_words(words)?.into_fixed()?;
    let rest = words.get(used..)?;
    if rest
        != [
            "cards", "of", "your", "library", "and", "separate", "them", "into", "a", "face",
            "down", "pile", "and", "a", "face", "up", "pile",
        ]
    {
        return None;
    }
    Some(DivvySequenceShape::ChosenOpponentFaceDownPiles {
        count: i32::try_from(count).ok()?,
        consumed_sentences: 3,
    })
}

// A pair of fixed, sequential library groups followed by a choice of group.
// Counts and visibility belong to the two producers, never a partition prompt.
fn parse_fixed_exile_piles(sentences: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    fn pile(words: &mut &[&str]) -> Option<(i32, bool)> {
        *words = words.strip_prefix(&["exile", "the", "top"])?;
        let (count, used) = crate::grammar::leaf::parse_leaf_number_prefix_words(words)?.into_fixed()?;
        *words = words.get(used..)?.strip_prefix(&["cards", "of", "your", "library", "in", "a", "face"])?;
        let down = match words.first()? { &"down" => true, &"up" => false, _ => return None };
        *words = words.get(1..)?.strip_prefix(&["pile"])?;
        Some((i32::try_from(count).ok()?, down))
    }
    if sentences.len() != 6 { return None; }
    let mut first = sentences[0].as_slice();
    let (first_count, first_face_down) = pile(&mut first)?;
    first = first.strip_prefix(&["then"])?;
    let (second_count, second_face_down) = pile(&mut first)?;
    if !first.is_empty() || !exact_sequence(&sentences[1..], &[
        &["an", "opponent", "chooses", "one", "of", "those", "piles"],
        &["put", "that", "pile", "into", "your", "graveyard"],
        &["look", "at", "the", "cards", "in", "the", "other", "pile"],
        &["you", "may", "cast", "a", "spell", "from", "among", "them", "without", "paying", "its", "mana", "cost"],
        &["put", "the", "rest", "into", "your", "hand"],
    ]) { return None; }
    Some(DivvySequenceShape::FixedExilePiles { first_count, second_count, first_face_down, second_face_down })
}

#[cfg(test)]
mod fixed_pile_tests {
    use super::*;
    #[test]
    fn fixed_piles_preserve_counts_visibility_and_complete_tail() {
        let text = "Exile the top four cards of your library in a face-down pile, then exile the top four cards of your library in a face-up pile. An opponent chooses one of those piles. Put that pile into your graveyard. Look at the cards in the other pile. You may cast a spell from among them without paying its mana cost. Put the rest into your hand.";
        for (text, first_count, second_count) in [(text.to_string(),4,4), (text.replacen("four", "two", 1).replacen("four", "six", 1),2,6)] {
            let lexed = crate::lexer::lex_line(&text, 0).unwrap();
            let sentences = crate::lexer::split_lexed_sentences(&lexed);
            assert_eq!(parse_divvy_sequence_shape(&sentences), Some(DivvySequenceShape::FixedExilePiles { first_count, second_count, first_face_down:true, second_face_down:false }), "{sentences:?}");
        }
        for bad in [text.replace("An opponent", "Each opponent"), text.replace("a spell", "two spells"), text.replace("Put the rest into your hand.", "Draw a card.")] {
            let lexed = crate::lexer::lex_line(&bad, 0).unwrap();
            assert!(parse_divvy_sequence_shape(&crate::lexer::split_lexed_sentences(&lexed)).is_none());
        }
    }
}
