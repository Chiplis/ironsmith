use super::*;

pub(super) fn is_unmodeled_predicate_words(filtered: &[&str]) -> bool {
    filtered == ["you", "gained", "life", "this", "turn"]
        || filtered == ["you", "dont", "cast", "it"]
        || filtered == ["it", "has", "odd", "number", "of", "counters", "on", "it"]
        || filtered == ["it", "has", "even", "number", "of", "counters", "on", "it"]
        || filtered == ["opponent", "lost", "life", "this", "turn"]
        || filtered == ["opponents", "lost", "life", "this", "turn"]
        || filtered == ["an", "opponent", "lost", "life", "this", "turn"]
        || filtered == ["this", "card", "in", "your", "graveyard"]
        || filtered == ["this", "artifact", "untapped"]
        || filtered == ["this", "has", "luck", "counter", "on", "it"]
        || filtered == ["it", "had", "revival", "counter", "on", "it"]
        || filtered == ["that", "creature", "would", "die", "this", "turn"]
        || filtered
            == [
                "this", "second", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered
            == [
                "this", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered
            == [
                "this",
                "fourth",
                "time",
                "this",
                "ability",
                "has",
                "triggered",
                "this",
                "turn",
            ]
        || filtered
            == [
                "this",
                "ability",
                "has",
                "been",
                "activated",
                "four",
                "or",
                "more",
                "times",
                "this",
                "turn",
            ]
        || filtered == ["it", "first", "combat", "phase", "of", "turn"]
        || filtered
            == [
                "you", "would", "begin", "your", "turn", "while", "this", "artifact", "is",
                "tapped",
            ]
        || filtered == ["player", "is", "dealt", "damage", "this", "way"]
        || filtered
            == [
                "two",
                "or",
                "more",
                "creatures",
                "are",
                "tied",
                "for",
                "least",
                "power",
            ]
        || filtered
            == [
                "card",
                "would",
                "be",
                "put",
                "into",
                "opponents",
                "graveyard",
                "from",
                "anywhere",
            ]
        || filtered == ["the", "number", "is", "odd"]
        || filtered == ["the", "number", "is", "even"]
        || filtered == ["number", "is", "odd"]
        || filtered == ["number", "is", "even"]
        || filtered == ["the", "number", "of", "permanents", "is", "odd"]
        || filtered == ["the", "number", "of", "permanents", "is", "even"]
        || filtered == ["number", "of", "permanents", "is", "odd"]
        || filtered == ["number", "of", "permanents", "is", "even"]
}
