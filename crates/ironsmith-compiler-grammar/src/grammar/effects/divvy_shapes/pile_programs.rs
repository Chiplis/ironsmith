//! Word shapes for multi-pile, labeled-pile, alternating-pick and per-player
//! exile-permission programs. Each shape names the leading sentences it
//! consumes; every remaining sentence is compiled independently by the
//! lowering in `effect_sentences/divvy/pile_programs.rs`.
use super::helpers::exact;

/// The object noun a "separates all <noun> they control" pile program splits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileObjectKind {
    Permanents,
    Creatures,
    Lands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PilePickActor {
    You,
    ChosenOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PilePickDestination {
    Hand,
    LibraryBottom,
}

/// One instruction of an alternating pick program over a revealed pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileOp {
    ChooseOpponent,
    Pick {
        actor: PilePickActor,
        destination: PilePickDestination,
    },
}

pub const MAX_PILE_OPS: usize = 8;

/// How long "players may play cards they exiled this way" lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExilePlayDuration {
    UntilYourNextTurn,
    UntilYourNextEndStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileProgramShape {
    /// "[Starting with you,] each player separates all permanents they
    /// control into three piles. Then each player chooses one of their piles
    /// at random and sacrifices those permanents." (Whims of the Fates)
    EachPlayerRandomPileSacrifice {
        piles: u8,
        kind: PileObjectKind,
        starting_with_you: bool,
        consumed_sentences: usize,
    },
    /// "Reveal the top five cards of your library. Put one of them into your
    /// hand. Then choose an opponent. They put one on the bottom of your
    /// library. ... Put the other into your hand." (Memories Returning)
    AlternatingRevealedPicks {
        count: u32,
        ops: [Option<PileOp>; MAX_PILE_OPS],
        rest: PilePickDestination,
        consumed_sentences: usize,
    },
    /// "[If your library has N or more cards in it,] target opponent looks at
    /// the top N cards of your library and separates them into two face-down
    /// piles. Exile one of those piles. Search the other pile for a card, put
    /// it into your hand, then shuffle the rest of that pile into your
    /// library." (Phyrexian Portal)
    OpponentFaceDownPilesExileOneSearchOther {
        count: u32,
        minimum_library: Option<u32>,
        consumed_sentences: usize,
    },
    /// "Each defending player divides all creatures without flying they
    /// control into a "left" pile and a "right" pile. Then, for each
    /// attacking creature you control, choose "left" or "right." That
    /// creature can't be blocked this combat except by creatures with flying
    /// and creatures in a pile with the chosen label." (Raging River)
    DefendingLabeledPilesBlockRestriction { consumed_sentences: usize },
    /// "Look at the top N cards of your library. Exile any number of them in
    /// a face-down pile and the rest in a face-up pile. Defending player
    /// chooses one of those piles. Put that pile into your hand."
    FaceDownFaceUpExilePilesDefenderChooses {
        count: u32,
        consumed_sentences: usize,
    },
    /// "Each player exiles the top N cards of their library. Until your next
    /// turn, players may play cards they exiled this way, and they can't play
    /// cards from their hand." (Memory Vessel; also the single-card
    /// "until your next end step, each player may play the card they exiled
    /// this way" form)
    EachPlayerExilesTopMayPlay {
        count: u32,
        duration: ExilePlayDuration,
        cant_play_from_hand: bool,
        consumed_sentences: usize,
    },
}

fn strip<'a, 'b>(words: &'a [&'b str], expected: &[&str]) -> Option<&'a [&'b str]> {
    if words.len() < expected.len() || !exact(&words[..expected.len()], expected) {
        return None;
    }
    Some(&words[expected.len()..])
}

fn strip_then<'a, 'b>(words: &'a [&'b str]) -> &'a [&'b str] {
    strip(words, &["then"]).unwrap_or(words)
}

fn number_prefix<'a, 'b>(words: &'a [&'b str]) -> Option<(u32, &'a [&'b str])> {
    let (count, used) =
        crate::grammar::leaf::parse_leaf_number_prefix_words(words)?.into_fixed()?;
    Some((count, words.get(used..)?))
}

pub(super) fn parse(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    parse_each_player_random_pile_sacrifice(sentences)
        .or_else(|| parse_alternating_revealed_picks(sentences))
        .or_else(|| parse_opponent_face_down_piles(sentences))
        .or_else(|| parse_defending_labeled_piles(sentences))
        .or_else(|| parse_face_down_face_up_exile_piles(sentences))
        .or_else(|| parse_each_player_exiles_top_may_play(sentences))
}

fn parse_kind(word: &str) -> Option<PileObjectKind> {
    match word {
        "permanents" => Some(PileObjectKind::Permanents),
        "creatures" => Some(PileObjectKind::Creatures),
        "lands" => Some(PileObjectKind::Lands),
        _ => None,
    }
}

fn parse_each_player_random_pile_sacrifice(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let [separate, choose, rest @ ..] = sentences else {
        return None;
    };
    let mut words = separate.as_slice();
    let starting_with_you = if let Some(tail) = strip(words, &["starting", "with", "you"]) {
        words = tail;
        true
    } else {
        false
    };
    let words = strip(words, &["each", "player"])?;
    let words = strip(words, &["separates"]).or_else(|| strip(words, &["divides"]))?;
    let words = strip(words, &["all"])?;
    let (kind_word, words) = words.split_first()?;
    let kind = parse_kind(kind_word)?;
    let words = strip(words, &["they", "control", "into"])?;
    let (piles, words) = number_prefix(words)?;
    if !(2..=u32::from(u8::MAX)).contains(&piles) || !exact(words, &["piles"]) {
        return None;
    }
    let choose = strip_then(choose);
    let choose = strip(
        choose,
        &[
            "each", "player", "chooses", "one", "of", "their", "piles", "at", "random", "and",
            "sacrifices", "those",
        ],
    )?;
    if !exact(choose, &[*kind_word]) {
        return None;
    }
    // The "(Piles can be empty.)" reminder, when it survives preprocessing,
    // restates the separation rule rather than adding an instruction.
    let consumed_sentences = if rest
        .first()
        .is_some_and(|words| exact(words, &["piles", "can", "be", "empty"]))
    {
        3
    } else {
        2
    };
    Some(PileProgramShape::EachPlayerRandomPileSacrifice {
        piles: u8::try_from(piles).ok()?,
        kind,
        starting_with_you,
        consumed_sentences,
    })
}

fn parse_pick(words: &[&str], opponent_chosen: bool) -> Option<PileOp> {
    let words = strip_then(words);
    let (actor, words) = if let Some(tail) = strip(words, &["they", "put", "one"]) {
        if !opponent_chosen {
            return None;
        }
        (PilePickActor::ChosenOpponent, tail)
    } else if let Some(tail) = strip(words, &["you", "put", "one"]) {
        (PilePickActor::You, tail)
    } else {
        (PilePickActor::You, strip(words, &["put", "one"])?)
    };
    let words = strip(words, &["of", "them"]).unwrap_or(words);
    let destination = if exact(words, &["into", "your", "hand"]) {
        PilePickDestination::Hand
    } else if exact(words, &["on", "the", "bottom", "of", "your", "library"]) {
        PilePickDestination::LibraryBottom
    } else {
        return None;
    };
    Some(PileOp::Pick { actor, destination })
}

fn parse_alternating_revealed_picks(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let (reveal, tail) = sentences.split_first()?;
    let words = strip(reveal, &["reveal", "the", "top"])?;
    let (count, words) = number_prefix(words)?;
    if !exact(words, &["cards", "of", "your", "library"]) {
        return None;
    }
    let mut ops = [None; MAX_PILE_OPS];
    let mut op_count = 0usize;
    let mut opponent_chosen = false;
    let mut opponent_picks = false;
    for (index, sentence) in tail.iter().enumerate() {
        let words = strip_then(sentence);
        let rest = if exact(words, &["put", "the", "other", "into", "your", "hand"])
            || exact(words, &["put", "the", "rest", "into", "your", "hand"])
        {
            Some(PilePickDestination::Hand)
        } else if exact(words, &["put", "the", "other", "on", "the", "bottom", "of", "your", "library"])
            || exact(words, &["put", "the", "rest", "on", "the", "bottom", "of", "your", "library"])
        {
            Some(PilePickDestination::LibraryBottom)
        } else {
            None
        };
        if let Some(rest) = rest {
            // Only an interleaved opponent pick distinguishes this program
            // from an ordinary "put one into your hand and the rest ..." pair.
            if !opponent_picks || op_count < 2 {
                return None;
            }
            return Some(PileProgramShape::AlternatingRevealedPicks {
                count,
                ops,
                rest,
                consumed_sentences: index + 2,
            });
        }
        let op = if exact(words, &["choose", "an", "opponent"]) && !opponent_chosen {
            opponent_chosen = true;
            PileOp::ChooseOpponent
        } else {
            let op = parse_pick(words, opponent_chosen)?;
            if matches!(op, PileOp::Pick { actor: PilePickActor::ChosenOpponent, .. }) {
                opponent_picks = true;
            }
            op
        };
        *ops.get_mut(op_count)? = Some(op);
        op_count += 1;
    }
    None
}

fn parse_opponent_face_down_piles(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let [look, exile, search, ..] = sentences else {
        return None;
    };
    let mut words = look.as_slice();
    let mut minimum_library = None;
    if let Some(tail) = strip(words, &["if", "your", "library", "has"]) {
        let (minimum, tail) = number_prefix(tail)?;
        words = strip(tail, &["or", "more", "cards", "in", "it"])?;
        minimum_library = Some(minimum);
    }
    let words = strip(words, &["target", "opponent", "looks", "at", "the", "top"])?;
    let (count, words) = number_prefix(words)?;
    if !exact(
        words,
        &[
            "cards", "of", "your", "library", "and", "separates", "them", "into", "two", "face",
            "down", "piles",
        ],
    ) || !exact(exile, &["exile", "one", "of", "those", "piles"])
        || !(exact(
            search,
            &[
                "search", "the", "other", "pile", "for", "a", "card", "put", "it", "into", "your",
                "hand", "then", "shuffle", "the", "rest", "of", "that", "pile", "into", "your",
                "library",
            ],
        ) || exact(
            search,
            &[
                "search", "the", "other", "pile", "for", "a", "card", "put", "it", "into", "your",
                "hand", "and", "shuffle", "the", "rest", "of", "that", "pile", "into", "your",
                "library",
            ],
        ))
    {
        return None;
    }
    Some(PileProgramShape::OpponentFaceDownPilesExileOneSearchOther {
        count,
        minimum_library,
        consumed_sentences: 3,
    })
}

fn parse_defending_labeled_piles(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let [divide, choose, restrict, ..] = sentences else {
        return None;
    };
    let divides = exact(
        divide,
        &[
            "each", "defending", "player", "divides", "all", "creatures", "without", "flying",
            "they", "control", "into", "a", "left", "pile", "and", "a", "right", "pile",
        ],
    );
    let chooses = exact(
        strip_then(choose),
        &[
            "for", "each", "attacking", "creature", "you", "control", "choose", "left", "or",
            "right",
        ],
    );
    let restricts = exact(
        restrict,
        &[
            "that", "creature", "cant", "be", "blocked", "this", "combat", "except", "by",
            "creatures", "with", "flying", "and", "creatures", "in", "a", "pile", "with", "the",
            "chosen", "label",
        ],
    );
    (divides && chooses && restricts)
        .then_some(PileProgramShape::DefendingLabeledPilesBlockRestriction { consumed_sentences: 3 })
}

fn parse_face_down_face_up_exile_piles(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let [look, exile, choose, put, ..] = sentences else {
        return None;
    };
    let words = strip(look, &["look", "at", "the", "top"])?;
    let (count, words) = number_prefix(words)?;
    if !exact(words, &["cards", "of", "your", "library"])
        || !exact(
            exile,
            &[
                "exile", "any", "number", "of", "them", "in", "a", "face", "down", "pile", "and",
                "the", "rest", "in", "a", "face", "up", "pile",
            ],
        )
        || !exact(choose, &["defending", "player", "chooses", "one", "of", "those", "piles"])
        || !exact(put, &["put", "that", "pile", "into", "your", "hand"])
    {
        return None;
    }
    Some(PileProgramShape::FaceDownFaceUpExilePilesDefenderChooses {
        count,
        consumed_sentences: 4,
    })
}

fn parse_each_player_exiles_top_may_play(sentences: &[Vec<&str>]) -> Option<PileProgramShape> {
    let [exile, permission, ..] = sentences else {
        return None;
    };
    let words = strip(exile, &["each", "player", "exiles", "the", "top"])?;
    let (count, words) = if let Some(tail) = strip(words, &["card"]) {
        (1, tail)
    } else {
        let (count, words) = number_prefix(words)?;
        (count, strip(words, &["cards"]).or_else(|| strip(words, &["card"]))?)
    };
    if !exact(words, &["of", "their", "library"]) {
        return None;
    }
    let (duration, cant_play_from_hand) = if exact(
        permission,
        &[
            "until", "your", "next", "turn", "players", "may", "play", "cards", "they", "exiled",
            "this", "way", "and", "they", "cant", "play", "cards", "from", "their", "hand",
        ],
    ) {
        (ExilePlayDuration::UntilYourNextTurn, true)
    } else if exact(
        permission,
        &[
            "until", "your", "next", "turn", "players", "may", "play", "cards", "they", "exiled",
            "this", "way",
        ],
    ) || exact(
        permission,
        &[
            "until", "your", "next", "turn", "each", "player", "may", "play", "the", "cards",
            "they", "exiled", "this", "way",
        ],
    ) {
        (ExilePlayDuration::UntilYourNextTurn, false)
    } else if exact(
        permission,
        &[
            "until", "your", "next", "end", "step", "each", "player", "may", "play", "the", "card",
            "they", "exiled", "this", "way",
        ],
    ) || exact(
        permission,
        &[
            "until", "your", "next", "end", "step", "each", "player", "may", "play", "the",
            "cards", "they", "exiled", "this", "way",
        ],
    ) {
        (ExilePlayDuration::UntilYourNextEndStep, false)
    } else {
        return None;
    };
    Some(PileProgramShape::EachPlayerExilesTopMayPlay {
        count,
        duration,
        cant_play_from_hand,
        consumed_sentences: 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(sentences: &[&'static str]) -> Vec<Vec<&'static str>> {
        sentences
            .iter()
            .map(|sentence| sentence.split(' ').collect())
            .collect()
    }

    #[test]
    fn pile_programs_recognize_package_shapes() {
        assert!(matches!(
            parse(&words(&[
                "starting with you each player separates all permanents they control into three piles",
                "then each player chooses one of their piles at random and sacrifices those permanents",
            ])),
            Some(PileProgramShape::EachPlayerRandomPileSacrifice {
                piles: 3,
                kind: PileObjectKind::Permanents,
                starting_with_you: true,
                consumed_sentences: 2,
            })
        ));
        assert!(matches!(
            parse(&words(&[
                "reveal the top five cards of your library",
                "put one of them into your hand",
                "then choose an opponent",
                "they put one on the bottom of your library",
                "then you put one into your hand",
                "then they put one on the bottom of your library",
                "put the other into your hand",
            ])),
            Some(PileProgramShape::AlternatingRevealedPicks {
                count: 5,
                rest: PilePickDestination::Hand,
                consumed_sentences: 7,
                ..
            })
        ));
        assert!(matches!(
            parse(&words(&[
                "if your library has ten or more cards in it target opponent looks at the top ten cards of your library and separates them into two face down piles",
                "exile one of those piles",
                "search the other pile for a card put it into your hand then shuffle the rest of that pile into your library",
            ])),
            Some(PileProgramShape::OpponentFaceDownPilesExileOneSearchOther {
                count: 10,
                minimum_library: Some(10),
                consumed_sentences: 3,
            })
        ));
        assert!(matches!(
            parse(&words(&[
                "each defending player divides all creatures without flying they control into a left pile and a right pile",
                "then for each attacking creature you control choose left or right",
                "that creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label",
            ])),
            Some(PileProgramShape::DefendingLabeledPilesBlockRestriction { consumed_sentences: 3 })
        ));
        assert!(matches!(
            parse(&words(&[
                "each player exiles the top seven cards of their library",
                "until your next turn players may play cards they exiled this way and they can't play cards from their hand",
            ])),
            Some(PileProgramShape::EachPlayerExilesTopMayPlay {
                count: 7,
                duration: ExilePlayDuration::UntilYourNextTurn,
                cant_play_from_hand: true,
                consumed_sentences: 2,
            })
        ));
    }
}
