use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalDelegatedGraveyardPartitionShape {
    pub pool_count: usize,
    pub subset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelegatedGraveyardPairPartitionShape {
    pub pool_count: usize,
    pub subset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealedTopDelegatedPartitionShape {
    pub pool_count: usize,
    pub subset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceExiledDelegatedPartitionShape {
    pub subset_count: usize,
}

/// Recognizes an activated-effect partition whose source collection was
/// already introduced by an exile activation cost.
pub fn parse_source_exiled_delegated_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceExiledDelegatedPartitionShape> {
    let sentences = split_lexed_sentences(tokens);
    let [choice, disposition] = sentences.as_slice() else {
        return None;
    };
    let choice = parser_token_word_refs(choice);
    let disposition = parser_token_word_refs(disposition);
    let [
        "an",
        "opponent",
        "chooses",
        subset_count,
        "of",
        "the",
        "exiled",
        "cards",
    ] = choice.as_slice()
    else {
        return None;
    };
    if disposition.as_slice()
        != [
            "you",
            "put",
            "that",
            "card",
            "on",
            "the",
            "bottom",
            "of",
            "your",
            "library",
            "and",
            "return",
            "the",
            "other",
            "to",
            "the",
            "battlefield",
            "tapped",
        ]
    {
        return None;
    }
    Some(SourceExiledDelegatedPartitionShape {
        subset_count: crate::util::parse_number_word_u32(subset_count)
            .and_then(|count| usize::try_from(count).ok())?,
    })
}

pub fn is_delegated_partition_program(tokens: &[OwnedLexToken]) -> bool {
    parse_delegated_graveyard_pair_partition_shape(tokens).is_some()
        || parse_conditional_delegated_graveyard_partition_shape(tokens).is_some()
        || parse_revealed_top_delegated_partition_shape(tokens).is_some()
        || parse_source_exiled_delegated_partition_shape(tokens).is_some()
}

/// Recognizes a revealed top-of-library collection partitioned into one
/// opponent-selected subset and the exact complement that is exiled with a
/// silver counter.
pub fn parse_revealed_top_delegated_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealedTopDelegatedPartitionShape> {
    let sentences = split_lexed_sentences(tokens);
    let [reveal, choice, disposition] = sentences.as_slice() else {
        return None;
    };
    let reveal = parser_token_word_refs(reveal);
    let choice = parser_token_word_refs(choice);
    let disposition = parser_token_word_refs(disposition);
    let [
        "reveal",
        "the",
        "top",
        pool_count,
        "cards",
        "of",
        "your",
        "library",
    ] = reveal.as_slice()
    else {
        return None;
    };
    let ["an", "opponent", "chooses", subset_count, "of", "them"] = choice.as_slice() else {
        return None;
    };
    if disposition.as_slice()
        != [
            "put", "that", "card", "into", "your", "hand", "and", "exile", "the", "other", "with",
            "a", "silver", "counter", "on", "it",
        ]
    {
        return None;
    }
    Some(RevealedTopDelegatedPartitionShape {
        pool_count: crate::util::parse_number_word_u32(pool_count)
            .and_then(|count| usize::try_from(count).ok())?,
        subset_count: crate::util::parse_number_word_u32(subset_count)
            .and_then(|count| usize::try_from(count).ok())?,
    })
}

/// Recognizes a target graveyard collection partitioned by an opponent into
/// one selected subset and its exact complement.
pub fn parse_delegated_graveyard_pair_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelegatedGraveyardPairPartitionShape> {
    let sentences = split_lexed_sentences(tokens);
    let [pool, choice, selected_move, complement_move] = sentences.as_slice() else {
        return None;
    };
    let pool = parser_token_word_refs(pool);
    let choice = parser_token_word_refs(choice);
    let selected_move = parser_token_word_refs(selected_move);
    let complement_move = parser_token_word_refs(complement_move);

    let [
        "choose",
        "up",
        "to",
        pool_count,
        "target",
        "creature",
        "cards",
        "in",
        "your",
        "graveyard",
    ] = pool.as_slice()
    else {
        return None;
    };
    let ["an", "opponent", "chooses", subset_count, "of", "them"] = choice.as_slice() else {
        return None;
    };
    if selected_move.as_slice() != ["return", "that", "card", "to", "your", "hand"]
        || complement_move.as_slice()
            != [
                "return",
                "the",
                "other",
                "to",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
            ]
    {
        return None;
    }

    Some(DelegatedGraveyardPairPartitionShape {
        pool_count: crate::util::parse_number_word_u32(pool_count)
            .and_then(|count| usize::try_from(count).ok())?,
        subset_count: crate::util::parse_number_word_u32(subset_count)
            .and_then(|count| usize::try_from(count).ok())?,
    })
}

/// Recognizes the complete four-sentence target-pool partition used by
/// effects of the form “choose targets; if ..., return them; otherwise an
/// opponent chooses some; leave those and move the rest”.  This is one
/// grammar program: the final remainder is scoped only to the alternative
/// branch and all demonstratives name sets introduced by this program.
pub fn parse_conditional_delegated_graveyard_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalDelegatedGraveyardPartitionShape> {
    let sentences = split_lexed_sentences(tokens);
    let [pool, condition, alternative, disposition] = sentences.as_slice() else {
        return None;
    };
    let pool = parser_token_word_refs(pool);
    let condition = parser_token_word_refs(condition);
    let alternative = parser_token_word_refs(alternative);
    let disposition = parser_token_word_refs(disposition);

    let [
        "choose",
        "up",
        "to",
        pool_count,
        "target",
        "cards",
        "in",
        "your",
        "graveyard",
    ] = pool.as_slice()
    else {
        return None;
    };
    let condition_is_planeswalker_control = condition.starts_with(&["if", "you", "control", "a"])
        && condition.ends_with(&[
            "planeswalker",
            "return",
            "those",
            "cards",
            "to",
            "your",
            "hand",
        ]);
    if !condition_is_planeswalker_control {
        return None;
    }
    let [
        "otherwise",
        "an",
        "opponent",
        "chooses",
        subset_count,
        "of",
        "them",
    ] = alternative.as_slice()
    else {
        return None;
    };
    if disposition.as_slice()
        != [
            "leave",
            "the",
            "chosen",
            "cards",
            "in",
            "your",
            "graveyard",
            "and",
            "put",
            "the",
            "rest",
            "into",
            "your",
            "hand",
        ]
    {
        return None;
    }

    Some(ConditionalDelegatedGraveyardPartitionShape {
        pool_count: crate::util::parse_number_word_u32(pool_count)
            .and_then(|count| usize::try_from(count).ok())?,
        subset_count: crate::util::parse_number_word_u32(subset_count)
            .and_then(|count| usize::try_from(count).ok())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_conditional_delegated_graveyard_partition_as_one_program() {
        let tokens = lex_line(
            "Choose up to four target cards in your graveyard. If you control a Bolas planeswalker, return those cards to your hand. Otherwise, an opponent chooses two of them. Leave the chosen cards in your graveyard and put the rest into your hand.",
            0,
        )
        .expect("program should lex");
        assert_eq!(
            parse_conditional_delegated_graveyard_partition_shape(&tokens),
            Some(ConditionalDelegatedGraveyardPartitionShape {
                pool_count: 4,
                subset_count: 2,
            })
        );
    }

    #[test]
    fn recognizes_delegated_graveyard_pair_as_one_partition_program() {
        let tokens = lex_line(
            "Choose up to two target creature cards in your graveyard. An opponent chooses one of them. Return that card to your hand. Return the other to the battlefield under your control.",
            0,
        )
        .expect("program should lex");
        assert_eq!(
            parse_delegated_graveyard_pair_partition_shape(&tokens),
            Some(DelegatedGraveyardPairPartitionShape {
                pool_count: 2,
                subset_count: 1,
            })
        );
    }

    #[test]
    fn recognizes_revealed_top_delegated_partition_as_one_program() {
        let tokens = lex_line(
            "Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the other with a silver counter on it.",
            0,
        )
        .expect("program should lex");
        assert_eq!(
            parse_revealed_top_delegated_partition_shape(&tokens),
            Some(RevealedTopDelegatedPartitionShape {
                pool_count: 2,
                subset_count: 1,
            })
        );
    }

    #[test]
    fn recognizes_source_exiled_delegated_partition_as_one_program() {
        let tokens = lex_line(
            "An opponent chooses one of the exiled cards. You put that card on the bottom of your library and return the other to the battlefield tapped.",
            0,
        )
        .expect("program should lex");
        assert_eq!(
            parse_source_exiled_delegated_partition_shape(&tokens),
            Some(SourceExiledDelegatedPartitionShape { subset_count: 1 })
        );
    }
}
