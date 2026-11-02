use winnow::combinator::{repeat_till};
use crate::grammar::primitives::peek;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenWordView};

#[path = "divvy_shapes/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "divvy_shapes/binary_program.rs"]
mod binary_program;
pub use binary_program::{BinaryPileCount, BinaryPileDestination, BinaryPilePartitioner, BinaryPileProducer, BinaryPileProgramShape};

#[path = "divvy_shapes/pile_programs.rs"]
mod pile_programs;
pub use pile_programs::{ExilePlayDuration, PileObjectKind, PileOp, PilePickActor, PilePickDestination, PileProgramShape};

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

/// The zones a lead's pool occupies once the lead sentences resolve: the
/// cards it searched for or revealed stay in the library, exiled cards are in
/// exile, moved cards are wherever the lead put them, and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DivvyPoolZones {
    pub library: bool,
    pub graveyard: bool,
    pub hand: bool,
    pub exile: bool,
    pub battlefield: bool,
}

impl DivvyPoolZones {
    pub fn zones(self) -> Vec<crate::zone::Zone> {
        use crate::zone::Zone;
        [
            (self.library, Zone::Library),
            (self.graveyard, Zone::Graveyard),
            (self.hand, Zone::Hand),
            (self.exile, Zone::Exile),
            (self.battlefield, Zone::Battlefield),
        ]
        .into_iter()
        .filter_map(|(present, zone)| present.then_some(zone))
        .collect()
    }

    fn is_empty(self) -> bool {
        self == Self::default()
    }

    fn noun(word: &str) -> Self {
        let mut zones = Self::default();
        match word {
            "library" | "libraries" => zones.library = true,
            "graveyard" | "graveyards" => zones.graveyard = true,
            "hand" | "hands" => zones.hand = true,
            "exile" => zones.exile = true,
            "battlefield" => zones.battlefield = true,
            _ => {}
        }
        zones
    }

    /// Read where the pool made by `lead` ends up. The last lead sentence
    /// that places cards decides: a destination ("into your graveyard",
    /// "onto the battlefield") wins, then an exile verb, then the zones the
    /// sentence draws its cards from; targets with no zone and no "cards"
    /// are permanents on the battlefield.
    fn from_lead(lead: &[Vec<&str>]) -> Option<Self> {
        for words in lead.iter().rev() {
            let destination = words
                .iter()
                .enumerate()
                .filter(|(_, word)| matches!(**word, "into" | "onto"))
                .filter_map(|(index, _)| {
                    words[index + 1..]
                        .iter()
                        .take(3)
                        .map(|word| Self::noun(word))
                        .find(|zones| !zones.is_empty())
                })
                .last();
            if let Some(zones) = destination {
                return Some(zones);
            }
            if words.iter().any(|word| matches!(*word, "exile" | "exiles" | "exiled")) {
                return Some(Self { exile: true, ..Self::default() });
            }
            let mut zones = Self::default();
            for word in words {
                let noun = Self::noun(word);
                zones.library |= noun.library;
                zones.graveyard |= noun.graveyard;
                zones.hand |= noun.hand;
                zones.battlefield |= noun.battlefield;
            }
            if !zones.is_empty() {
                return Some(zones);
            }
            if words.contains(&"target") && !words.iter().any(|word| matches!(*word, "card" | "cards")) {
                return Some(Self { battlefield: true, ..Self::default() });
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivvySequenceShape {
    BinaryCards(BinaryPileProgramShape),
    /// Multi-pile, labeled-pile, alternating-pick and per-player exile
    /// permission programs (see `pile_programs`).
    PileProgram(PileProgramShape),
    FixedExilePiles {
        first_count: i32,
        second_count: i32,
        first_face_down: bool,
        second_face_down: bool,
    },
    /// "Choose an opponent. They look at the top N cards of your library and
    /// separate them into a face-down pile and a face-up pile. Put one pile
    /// into your hand and the other into your graveyard." Any following
    /// sentences are independent.
    ChosenOpponentFaceDownPiles {
        count: i32,
        consumed_sentences: usize,
    },
    /// "Search your library and graveyard for up to four creature cards ...
    /// An opponent chooses two of those cards. Shuffle the chosen cards into
    /// your library and put the rest onto the battlefield." Every sentence
    /// after the third compiles on its own. `fused` marks a program that
    /// arrived as one input whose sentences the lowering splits itself.
    SearchFourCreatureCards { fused: bool },
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
    /// The Fact or Fiction split whose other pile goes on the bottom of the
    /// library in any order (Jace, Architect of Thought).
    RevealTopPilesOtherOnBottom,
    ExileCreatureCardsFromGraveyards,
    /// "<lead sentences>. An opponent chooses one of them. Put the chosen
    /// card into your hand and the other into your graveyard[, then
    /// shuffle]." The lead sentences make the pool and compile on their own.
    ChooseOneOfThem {
        lead_sentences: usize,
        pool: DivvyPoolZones,
        shuffle: bool,
    },
    SearchFourDifferentNames {
        chooser: DivvyChooserShape,
        rest: DivvyRestDestinationShape,
        shuffle: bool,
    },
    SearchFourDifferentPowers,
    /// "<lead sentences>. Target opponent chooses one. Put that card into
    /// your hand and the rest into your graveyard. [Then shuffle.]"
    TargetOpponentChoosesOne {
        lead_sentences: usize,
        pool: DivvyPoolZones,
        shuffle: bool,
    },
}

pub fn parse_divvy_sequence_shape(sentences: &[&[OwnedLexToken]]) -> Option<DivvySequenceShape> {
    if let Some(program) = binary_program::parse(sentences) {
        return Some(DivvySequenceShape::BinaryCards(program));
    }
    {
        let words = sentences
            .iter()
            .map(|tokens| TokenWordView::new(tokens).to_word_refs())
            .collect::<Vec<_>>();
        if let Some(program) = pile_programs::parse(&words) {
            return Some(DivvySequenceShape::PileProgram(program));
        }
    }
    let sentence_words = sentences
        .iter()
        .map(|tokens| TokenWordView::new(tokens).to_word_refs())
        .collect::<Vec<_>>();

    // Both fixed shapes match every word of the sentences they consume.
    let fixed = crate::word_claims::speculative_keep_if(
        || {
            parse_fixed_exile_piles(&sentence_words)
                .map(|shape| (shape, 6))
                .or_else(|| {
                    parse_chosen_opponent_face_down_piles(&sentence_words).map(|shape| (shape, 3))
                })
        },
        Option::is_some,
    );
    if let Some((shape, consumed)) = fixed {
        for tokens in sentences.get(..consumed).unwrap_or(&[]) {
            crate::word_claims::claim_tokens(tokens);
        }
        return Some(shape);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::SearchLibraryGraveyardExileRemainderToTop);
    }

    // A program that arrives as one fused input reads its own sentences.
    if let [fused] = sentences {
        let parts = crate::lexer::split_lexed_sentences(fused);
        if parts.len() > 1 {
            let part_words = parts
                .iter()
                .map(|tokens| TokenWordView::new(tokens).to_word_refs())
                .collect::<Vec<_>>();
            if crate::word_claims::speculative_keep_if(
                || parse_search_four_creature_cards(&part_words),
                Option::is_some,
            )
            .is_some()
            {
                return Some(DivvySequenceShape::SearchFourCreatureCards { fused: true });
            }
        }
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::ExchangeCreatureControl);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::DestroyChosenCreaturePile);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::GraveyardCreaturePiles);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_opponent_creature_piles_sacrifice(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_permanent_piles_sacrifice(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::DefendingCreaturePilesBlock);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_creature_piles_attack(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::LandPiles);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_exile_and_separate_piles(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_reveal_top_opponent_piles(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::ExileCreatureCardsFromGraveyards);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_search_four_creature_cards(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_choose_one_of_them(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_search_different_names(&sentence_words), Option::is_some) {
        return Some(shape);
    }

    if crate::word_claims::speculative_keep_if(|| exact_sequence(
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
    ), |matched| *matched) {
        return Some(DivvySequenceShape::SearchFourDifferentPowers);
    }

    if let Some(shape) = crate::word_claims::speculative_keep_if(|| parse_target_opponent_chooses_one(&sentence_words), Option::is_some) {
        return Some(shape);
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
                "put",
                "one",
                "pile",
                "into",
                "your",
                "hand",
                "and",
                "the",
                "other",
                "into",
                "your",
                "graveyard",
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
        let (count, used) =
            crate::grammar::leaf::parse_leaf_number_prefix_words(words)?.into_fixed()?;
        *words = words
            .get(used..)?
            .strip_prefix(&["cards", "of", "your", "library", "in", "a", "face"])?;
        let down = match words.first()? {
            &"down" => true,
            &"up" => false,
            _ => return None,
        };
        *words = words.get(1..)?.strip_prefix(&["pile"])?;
        Some((i32::try_from(count).ok()?, down))
    }
    if sentences.len() != 6 {
        return None;
    }
    let mut first = sentences[0].as_slice();
    let (first_count, first_face_down) = pile(&mut first)?;
    first = first.strip_prefix(&["then"])?;
    let (second_count, second_face_down) = pile(&mut first)?;
    if !first.is_empty()
        || !exact_sequence(
            &sentences[1..],
            &[
                &["an", "opponent", "chooses", "one", "of", "those", "piles"],
                &["put", "that", "pile", "into", "your", "graveyard"],
                &["look", "at", "the", "cards", "in", "the", "other", "pile"],
                &[
                    "you", "may", "cast", "a", "spell", "from", "among", "them", "without",
                    "paying", "its", "mana", "cost",
                ],
                &["put", "the", "rest", "into", "your", "hand"],
            ],
        )
    {
        return None;
    }
    Some(DivvySequenceShape::FixedExilePiles {
        first_count,
        second_count,
        first_face_down,
        second_face_down,
    })
}

#[cfg(test)]
mod fixed_pile_tests {
    use super::*;
    #[test]
    fn fixed_piles_preserve_counts_visibility_and_complete_tail() {
        let text = "Exile the top four cards of your library in a face-down pile, then exile the top four cards of your library in a face-up pile. An opponent chooses one of those piles. Put that pile into your graveyard. Look at the cards in the other pile. You may cast a spell from among them without paying its mana cost. Put the rest into your hand.";
        for (text, first_count, second_count) in [
            (text.to_string(), 4, 4),
            (
                text.replacen("four", "two", 1).replacen("four", "six", 1),
                2,
                6,
            ),
        ] {
            let lexed = crate::lexer::lex_line(&text, 0).unwrap();
            let sentences = crate::lexer::split_lexed_sentences(&lexed);
            assert_eq!(
                parse_divvy_sequence_shape(&sentences),
                Some(DivvySequenceShape::BinaryCards(BinaryPileProgramShape {
                    producer: BinaryPileProducer::FaceDownThenFaceUpExile {
                        first: first_count, second: second_count,
                    },
                    partitioner: BinaryPilePartitioner::You,
                    reveal_pool: false,
                    destination: BinaryPileDestination::ChosenToGraveyardCastFromOtherRestToHand,
                    consumed_sentences: 6,
                })),
                "{sentences:?}"
            );
        }
        for bad in [
            text.replace("An opponent", "Each opponent"),
            text.replace("a spell", "two spells"),
            text.replace("Put the rest into your hand.", "Draw a card."),
        ] {
            let lexed = crate::lexer::lex_line(&bad, 0).unwrap();
            assert!(
                parse_divvy_sequence_shape(&crate::lexer::split_lexed_sentences(&lexed)).is_none()
            );
        }
    }
}

// Each reader below matches every word of the sentences it models; lead
// sentences that the lowering compiles on its own are read there.

/// "Each opponent separates the creatures they control into two piles. For
/// each opponent, you choose one of their piles. Each opponent sacrifices the
/// creatures in their chosen pile." (Make an Example)
fn parse_opponent_creature_piles_sacrifice(
    sentence_words: &[Vec<&str>],
) -> Option<DivvySequenceShape> {
    exact_sequence(
        without_piles_reminder(sentence_words),
        &[
            &[
                "each", "opponent", "separates", "the", "creatures", "they", "control", "into",
                "two", "piles",
            ],
            &["for", "each", "opponent", "you", "choose", "one", "of", "their", "piles"],
            &[
                "each", "opponent", "sacrifices", "the", "creatures", "in", "their", "chosen",
                "pile",
            ],
        ],
    )
    .then_some(DivvySequenceShape::OpponentCreaturePilesSacrifice)
}

/// "Separate all permanents target player controls into two piles. That
/// player sacrifices all permanents in the pile of their choice." (Liliana of
/// the Veil)
fn parse_permanent_piles_sacrifice(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    exact_sequence(
        without_piles_reminder(sentence_words),
        &[
            &[
                "separate", "all", "permanents", "target", "player", "controls", "into", "two",
                "piles",
            ],
            &[
                "that", "player", "sacrifices", "all", "permanents", "in", "the", "pile", "of",
                "their", "choice",
            ],
        ],
    )
    .then_some(DivvySequenceShape::PermanentPilesSacrifice)
}

/// "Separate all creatures that player controls into two piles. Only
/// creatures in the pile of their choice can attack this turn." (Fight or
/// Flight)
fn parse_creature_piles_attack(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    exact_sequence(
        without_piles_reminder(sentence_words),
        &[
            &[
                "separate", "all", "creatures", "that", "player", "controls", "into", "two",
                "piles",
            ],
            &[
                "only", "creatures", "in", "the", "pile", "of", "their", "choice", "can",
                "attack", "this", "turn",
            ],
        ],
    )
    .then_some(DivvySequenceShape::CreaturePilesAttack)
}

/// "<Exile ...> and separate them into two piles. An opponent chooses one of
/// those piles. Put that pile into your hand and the other into your
/// graveyard." (Split the Spoils). The exile clause is compiled on its own,
/// so any exile wording makes the pool.
fn parse_exile_and_separate_piles(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    const SEPARATE: &[&str] = &["and", "separate", "them", "into", "two", "piles"];
    let [exile, choose, put] = without_piles_reminder(sentence_words) else {
        return None;
    };
    let head_len = exile.len().checked_sub(SEPARATE.len())?;
    (head_len > 1
        && prefix(exile, &["exile"])
        && exact(&exile[head_len..], SEPARATE)
        && exact(choose, &["an", "opponent", "chooses", "one", "of", "those", "piles"])
        && exact(
            put,
            &[
                "put", "that", "pile", "into", "your", "hand", "and", "the", "other", "into",
                "your", "graveyard",
            ],
        ))
    .then_some(DivvySequenceShape::ExilePermanentCardsPile)
}

/// "Reveal the top N cards of your library. An opponent separates those cards
/// into two piles. Put one pile into your hand and the other into your
/// graveyard | on the bottom of your library in any order." The reveal is
/// compiled on its own.
fn parse_reveal_top_opponent_piles(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let [reveal, separate, put] = without_piles_reminder(sentence_words) else {
        return None;
    };
    if !(prefix(reveal, &["reveal", "the", "top"])
        && phrase_anywhere(reveal, &["cards", "of", "your", "library"])
        && exact(
            separate,
            &["an", "opponent", "separates", "those", "cards", "into", "two", "piles"],
        ))
    {
        return None;
    }
    if exact(
        put,
        &[
            "put", "one", "pile", "into", "your", "hand", "and", "the", "other", "into", "your",
            "graveyard",
        ],
    ) {
        Some(DivvySequenceShape::RevealTopPiles)
    } else if exact(
        put,
        &[
            "put", "one", "pile", "into", "your", "hand", "and", "the", "other", "on", "the",
            "bottom", "of", "your", "library", "in", "any", "order",
        ],
    ) {
        Some(DivvySequenceShape::RevealTopPilesOtherOnBottom)
    } else {
        None
    }
}

/// "Search your library and graveyard for up to four creature cards with
/// different names that each have mana value X or less and reveal them. An
/// opponent chooses two of those cards. Shuffle the chosen cards into your
/// library and put the rest onto the battlefield." (Ecological Appreciation).
/// The search is compiled on its own, as is every following sentence.
fn parse_search_four_creature_cards(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let [search, choose, put, ..] = sentence_words else {
        return None;
    };
    (prefix(
        search,
        &[
            "search", "your", "library", "and", "graveyard", "for", "up", "to", "four",
            "creature", "cards",
        ],
    ) && phrase_anywhere(search, &["different", "names"])
        && phrase_anywhere(search, &["mana", "value", "x", "or", "less"])
        && phrase_anywhere(search, &["reveal", "them"])
        && exact(choose, &["an", "opponent", "chooses", "two", "of", "those", "cards"])
        && exact(
            put,
            &[
                "shuffle", "the", "chosen", "cards", "into", "your", "library", "and", "put",
                "the", "rest", "onto", "the", "battlefield",
            ],
        ))
    .then_some(DivvySequenceShape::SearchFourCreatureCards { fused: false })
}

/// "<lead>. An opponent chooses one of them. Put the chosen card into your
/// hand and the other into your graveyard[, then shuffle]." (Burning-Rune
/// Demon)
fn parse_choose_one_of_them(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let lead_sentences =
        find_exact_sentence(sentence_words, 1, &["an", "opponent", "chooses", "one", "of", "them"])?;
    let shuffle = disposition_then_shuffle(
        &sentence_words[lead_sentences + 1..],
        &[
            "put", "the", "chosen", "card", "into", "your", "hand", "and", "the", "other", "into",
            "your", "graveyard",
        ],
    )?;
    let pool = DivvyPoolZones::from_lead(&sentence_words[..lead_sentences])?;
    Some(DivvySequenceShape::ChooseOneOfThem {
        lead_sentences,
        pool,
        shuffle,
    })
}

/// "Search your library for up to four ... cards with different names and
/// reveal them. (Target) opponent chooses two of those cards. Put the chosen
/// cards into your graveyard and the rest into your hand | onto the
/// battlefield tapped[, then shuffle | . Then shuffle.]" (Gifts Ungiven,
/// Realms Uncharted, Elemental Teachings). The search is compiled on its own.
fn parse_search_different_names(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let [search, choose, rest @ ..] = sentence_words else {
        return None;
    };
    if !(prefix(search, &["search", "your", "library", "for", "up", "to", "four"])
        && phrase_anywhere(search, &["cards", "with", "different", "names"])
        && phrase_anywhere(search, &["reveal", "them"]))
    {
        return None;
    }
    let chooser = if exact(
        choose,
        &["target", "opponent", "chooses", "two", "of", "those", "cards"],
    ) {
        DivvyChooserShape::TargetOpponent
    } else if exact(choose, &["an", "opponent", "chooses", "two", "of", "those", "cards"]) {
        DivvyChooserShape::Opponent
    } else {
        return None;
    };
    const TO_GRAVEYARD: &[&str] = &[
        "put", "the", "chosen", "cards", "into", "your", "graveyard", "and", "the", "rest",
    ];
    let destinations: [(&[&str], DivvyRestDestinationShape); 2] = [
        (&["into", "your", "hand"], DivvyRestDestinationShape::Hand),
        (
            &["onto", "the", "battlefield", "tapped"],
            DivvyRestDestinationShape::BattlefieldTapped,
        ),
    ];
    destinations.into_iter().find_map(|(destination, rest_shape)| {
        let expected = [TO_GRAVEYARD, destination].concat();
        crate::word_claims::speculative_keep_if(
            || disposition_then_shuffle(rest, &expected),
            Option::is_some,
        )
        .map(|shuffle| DivvySequenceShape::SearchFourDifferentNames {
            chooser,
            rest: rest_shape,
            shuffle,
        })
    })
}

/// "<lead>. Target opponent chooses one. Put that card into your hand and the
/// rest into your graveyard. [Then shuffle.]" (Intuition)
fn parse_target_opponent_chooses_one(sentence_words: &[Vec<&str>]) -> Option<DivvySequenceShape> {
    let lead_sentences =
        find_exact_sentence(sentence_words, 1, &["target", "opponent", "chooses", "one"])?;
    let shuffle = disposition_then_shuffle(
        &sentence_words[lead_sentences + 1..],
        &[
            "put", "that", "card", "into", "your", "hand", "and", "the", "rest", "into", "your",
            "graveyard",
        ],
    )?;
    let pool = DivvyPoolZones::from_lead(&sentence_words[..lead_sentences])?;
    Some(DivvySequenceShape::TargetOpponentChoosesOne {
        lead_sentences,
        pool,
        shuffle,
    })
}
