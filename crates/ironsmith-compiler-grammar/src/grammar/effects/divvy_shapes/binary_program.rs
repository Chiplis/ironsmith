//! Finite library pools, two-way partitions, and their complete dispositions.
//! No card identity or presentation label participates in recognition.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPileCount {
    Fixed(i32),
    XPlus(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPileProducer {
    TopLibrary(BinaryPileCount),
    SequentialFaceDownExile { first: i32, second: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPilePartitioner {
    You,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPileDestination {
    HandAndGraveyard,
    OneToHandAndPoolToBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryPileProgramShape {
    pub producer: BinaryPileProducer,
    pub partitioner: BinaryPilePartitioner,
    pub reveal_pool: bool,
    pub destination: BinaryPileDestination,
    pub consumed_sentences: usize,
}

fn surface_is_complete(tokens: &[OwnedLexToken]) -> bool {
    use crate::lexer::TokenKind;
    tokens.iter().enumerate().all(|(index, token)| match token.kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Comma => true,
        TokenKind::Period => index + 1 == tokens.len(),
        TokenKind::Dash => index > 0
            && tokens[index - 1].is_word("face")
            && tokens.get(index + 1).is_some_and(|next| next.is_word("up") || next.is_word("down")),
        _ => false,
    })
}

pub(super) fn parse(sentences: &[&[OwnedLexToken]]) -> Option<BinaryPileProgramShape> {
    if let Some(program) = parse_sequential_exile(sentences) { return Some(program); }
    let first = *sentences.first()?;
    let words = TokenWordView::new(first).to_word_refs();
    let (partitioner, reveal_pool, mut remaining) =
        if let Some(rest) = words.strip_prefix(&["target", "opponent", "looks", "at", "the", "top"]) {
            (BinaryPilePartitioner::TargetOpponent, false, rest)
        } else if let Some(rest) = words.strip_prefix(&["look", "at", "the", "top"]) {
            (BinaryPilePartitioner::You, false, rest)
        } else if let Some(rest) = words.strip_prefix(&["reveal", "the", "top"]) {
            (BinaryPilePartitioner::You, true, rest)
        } else { return None; };
    let x = remaining.starts_with(&["x", "plus"]);
    if x { remaining = &remaining[2..]; }
    let (count, used) = crate::grammar::leaf::parse_leaf_number_prefix_words(remaining)?.into_fixed()?;
    let count = i32::try_from(count).ok()?;
    remaining = remaining.get(used..)?.strip_prefix(&["cards", "of", "your", "library", "and"])?;
    remaining = remaining.strip_prefix(&[match partitioner {
        BinaryPilePartitioner::You => "separate",
        BinaryPilePartitioner::TargetOpponent => "separates",
    }, "them", "into"])?;
    let expected_partition: &[&str] = if reveal_pool {
        &["two", "piles"]
    } else {
        &["a", "face", "down", "pile", "and", "a", "face", "up", "pile"]
    };
    if remaining != expected_partition { return None; }
    let disposition_index = if partitioner == BinaryPilePartitioner::You {
        let choice = TokenWordView::new(sentences.get(1)?).to_word_refs();
        if !matches!(choice.as_slice(),
            ["an", "opponent", "chooses", "one", "of", "those" | "the", "piles"])
        { return None; }
        2
    } else { 1 };
    let put = TokenWordView::new(sentences.get(disposition_index)?).to_word_refs();
    let destination = if put.as_slice() == ["put",
        if partitioner == BinaryPilePartitioner::You { "that" } else { "one" },
        "pile", "into", "your", "hand", "and", "the", "other", "into", "your", "graveyard"]
    {
        BinaryPileDestination::HandAndGraveyard
    } else if reveal_pool && put.as_slice() == [
        "put", "a", "card", "from", "the", "chosen", "pile", "into", "your", "hand", "then",
        "put", "all", "other", "cards", "revealed", "this", "way", "on", "the", "bottom", "of",
        "your", "library", "in", "any", "order",
    ] {
        BinaryPileDestination::OneToHandAndPoolToBottom
    } else { return None; };
    let consumed_sentences = disposition_index + 1;
    if !sentences[..consumed_sentences].iter().all(|tokens| surface_is_complete(tokens)) {
        return None;
    }
    Some(BinaryPileProgramShape {
        producer: BinaryPileProducer::TopLibrary(if x { BinaryPileCount::XPlus(count) } else { BinaryPileCount::Fixed(count) }),
        partitioner, reveal_pool, destination, consumed_sentences,
    })
}

fn parse_sequential_exile(sentences: &[&[OwnedLexToken]]) -> Option<BinaryPileProgramShape> {
    let [first, look, choose, put, ..] = sentences else { return None; };
    let words = TokenWordView::new(first).to_word_refs();
    let mut words = words.as_slice();
    fn read_pile(words: &mut &[&str], article: &str) -> Option<i32> {
        *words = words.strip_prefix(&["exile", "the", "top"])?;
        let (count, used) = crate::grammar::leaf::parse_leaf_number_prefix_words(words)?.into_fixed()?;
        *words = words.get(used..)?.strip_prefix(&["cards", "of", "your", "library", "in"])?;
        if words.first().copied()? != article { return None; }
        *words = words.get(1..)?.strip_prefix(&["face", "down", "pile"])?;
        i32::try_from(count).ok()
    }
    let first_count = read_pile(&mut words, "a")?;
    words = words.strip_prefix(&["then"])?;
    let second_count = read_pile(&mut words, "another")?;
    if !words.is_empty() { return None; }
    let look = TokenWordView::new(look).to_word_refs();
    let choose = TokenWordView::new(choose).to_word_refs();
    let put = TokenWordView::new(put).to_word_refs();
    if look.as_slice() != ["look", "at", "the", "cards", "in", "each", "pile", "then", "turn", "a", "pile", "of", "your", "choice", "face", "up"]
        || !matches!(choose.as_slice(), ["an", "opponent", "chooses", "one", "of", "the" | "those", "piles"])
        || put.as_slice() != ["put", "that", "pile", "into", "your", "hand", "and", "the", "other", "into", "your", "graveyard"]
        || !sentences[..4].iter().all(|tokens| surface_is_complete(tokens))
    { return None; }
    Some(BinaryPileProgramShape {
        producer: BinaryPileProducer::SequentialFaceDownExile { first: first_count, second: second_count },
        partitioner: BinaryPilePartitioner::You, reveal_pool: false,
        destination: BinaryPileDestination::HandAndGraveyard, consumed_sentences: 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(text: &str) -> Option<BinaryPileProgramShape> {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        parse(&crate::lexer::split_lexed_sentences(&tokens))
    }
    const REVEAL: &str = "Reveal the top five cards of your library and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.";
    #[test]
    fn quantities_roles_visibility_and_destinations_are_typed() {
        let revealed = read(REVEAL).unwrap();
        assert_eq!(revealed.producer, BinaryPileProducer::TopLibrary(BinaryPileCount::Fixed(5)));
        assert!(revealed.reveal_pool);
        assert_eq!(revealed.partitioner, BinaryPilePartitioner::You);
        assert_eq!(revealed.destination, BinaryPileDestination::HandAndGraveyard);
        assert_eq!(revealed.consumed_sentences, 3);
        assert_eq!(read(&REVEAL.replace("five", "X plus two")).unwrap().producer, BinaryPileProducer::TopLibrary(BinaryPileCount::XPlus(2)));
        let private = read("Target opponent looks at the top four cards of your library and separates them into a face-down pile and a face-up pile. Put one pile into your hand and the other into your graveyard.").unwrap();
        assert_eq!(private.partitioner, BinaryPilePartitioner::TargetOpponent);
        assert!(!private.reveal_pool);
        assert_eq!(private.consumed_sentences, 2);
        let one = read(&REVEAL.replace("Put that pile into your hand and the other into your graveyard.", "Put a card from the chosen pile into your hand, then put all other cards revealed this way on the bottom of your library in any order.")).unwrap();
        assert_eq!(one.destination, BinaryPileDestination::OneToHandAndPoolToBottom);
    }
    #[test]
    fn unsupported_pile_domains_and_nonword_payloads_are_not_erased() {
        for bad in [
            REVEAL.replace("two piles", "three piles"),
            REVEAL.replace("two piles", "two piles at random"),
            REVEAL.replace("your library", "target player's library"),
            REVEAL.replace("An opponent", "Each opponent"),
            REVEAL.replace("graveyard", "exile"),
            REVEAL.replace("five cards", "five {U} cards"),
            REVEAL.replace("five cards", "five + cards"),
            REVEAL.replace("two piles.", "two piles;"),
            REVEAL.replace("graveyard.", "graveyard and draw a card."),
            REVEAL.replace("two piles.", "two piles (1)."),
        ] { assert!(read(&bad).is_none(), "{bad}"); }
    }

    #[test]
    fn sequential_exile_counts_and_chosen_exposure_keep_complete_clauses() {
        let text = "Exile the top two cards of your library in a face-down pile, then exile the top six cards of your library in another face-down pile. Look at the cards in each pile, then turn a pile of your choice face up. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard. You lose 3 life.";
        let program = read(text).unwrap();
        assert_eq!(program.producer, BinaryPileProducer::SequentialFaceDownExile { first: 2, second: 6 });
        assert_eq!(program.consumed_sentences, 4);
        for invalid in [
            text.replace("then exile", "exile"),
            text.replace("your choice", "an opponent's choice"),
            text.replace("each pile", "the first pile"),
            text.replace("another face-down", "another face-up"),
            text.replace("face up.", "face up {B}."),
        ] { assert!(read(&invalid).is_none(), "{invalid}"); }
    }
}
