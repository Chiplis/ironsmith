use crate::cards::builders::CardTextError;

use super::super::lexer::{OwnedLexToken, TokenWordView};

struct UnsupportedWordRule {
    phrase: &'static [&'static str],
    message: &'static str,
}

const UNSUPPORTED_STARTS_WITH_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &["partner", "with"],
        message: "unsupported partner-with keyword line [rule=partner-with-keyword-line]",
    },
    UnsupportedWordRule {
        phrase: &[
            "the", "first", "creature", "spell", "you", "cast", "each", "turn", "costs",
        ],
        message: "unsupported first-spell cost modifier mechanic",
    },
    UnsupportedWordRule {
        phrase: &[
            "once", "each", "turn", "you", "may", "play", "a", "card", "from", "exile",
        ],
        message: "unsupported static clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "prevent", "the", "next", "1", "damage", "that", "would", "be", "dealt", "to", "any",
            "target", "this", "turn", "by", "red", "sources",
        ],
        message: "unsupported trailing prevent-next damage clause",
    },
    UnsupportedWordRule {
        phrase: &["ninjutsu", "abilities", "you", "activate", "cost"],
        message: "unsupported marker keyword with non-keyword tail",
    },
];

const UNSUPPORTED_CONTAINS_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &[
            "same", "name", "as", "another", "card", "in", "their", "hand",
        ],
        message: "unsupported same-name-as-another-in-hand discard clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "enters", "tapped", "and", "doesnt", "untap", "during", "your", "untap", "step",
        ],
        message: "unsupported mixed enters-tapped and negated-untap clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "prevent",
            "all",
            "combat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "this",
            "turn",
            "by",
            "creatures",
            "with",
            "power",
        ],
        message: "unsupported prevent-all-combat-damage clause tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "put",
            "one",
            "of",
            "them",
            "into",
            "your",
            "hand",
            "and",
            "the",
            "rest",
            "into",
            "your",
            "graveyard",
        ],
        message: "unsupported multi-destination put clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "assigns",
            "no",
            "combat",
            "damage",
            "this",
            "turn",
            "and",
            "defending",
            "player",
            "loses",
        ],
        message: "unsupported assigns-no-combat-damage clause",
    },
    UnsupportedWordRule {
        phrase: &["of", "defending", "players", "choice"],
        message: "unsupported defending-players-choice clause",
    },
    UnsupportedWordRule {
        phrase: &["if", "you", "sacrifice", "an", "island", "this", "way"],
        message: "unsupported if-you-sacrifice-an-island-this-way clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "create", "a", "token", "thats", "a", "copy", "of", "that", "aura", "attached", "to",
            "that", "creature",
        ],
        message: "unsupported aura-copy attachment fanout clause",
    },
    UnsupportedWordRule {
        phrase: &["target", "face", "down", "creature"],
        message: "unsupported face-down clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "with",
            "islandwalk",
            "can",
            "be",
            "blocked",
            "as",
            "though",
            "they",
            "didnt",
            "have",
            "islandwalk",
        ],
        message: "unsupported landwalk override clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "with",
            "power",
            "or",
            "toughness",
            "1",
            "or",
            "less",
            "cant",
            "be",
            "blocked",
        ],
        message: "unsupported power-or-toughness cant-be-blocked subject",
    },
    UnsupportedWordRule {
        phrase: &[
            "discard",
            "up",
            "to",
            "two",
            "permanents",
            "then",
            "draw",
            "that",
            "many",
            "cards",
        ],
        message: "unsupported discard qualifier clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "if", "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half",
            "your", "starting", "life", "total", "plus", "one",
        ],
        message: "unsupported predicate",
    },
    UnsupportedWordRule {
        phrase: &[
            "then",
            "sacrifices",
            "all",
            "creatures",
            "they",
            "control",
            "then",
            "puts",
            "all",
            "cards",
            "they",
            "exiled",
            "this",
            "way",
            "onto",
            "the",
            "battlefield",
        ],
        message: "unsupported each-player exile/sacrifice/return-this-way clause",
    },
    UnsupportedWordRule {
        phrase: &["if", "this", "creature", "isnt", "saddled", "this", "turn"],
        message: "unsupported saddled conditional tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "put", "a", "card", "from", "among", "them", "into", "your", "hand", "this", "turn",
        ],
        message: "unsupported looked-card fallback tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "if",
            "the",
            "sacrificed",
            "creature",
            "was",
            "a",
            "hamster",
            "this",
            "turn",
        ],
        message: "unsupported predicate",
    },
];

const UNSUPPORTED_EQUALS_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &[
            "creatures",
            "you",
            "control",
            "have",
            "haste",
            "and",
            "attack",
            "each",
            "combat",
            "if",
            "able",
        ],
        message: "unsupported anthem subject",
    },
    UnsupportedWordRule {
        phrase: &[
            "you", "may", "play", "any", "number", "of", "lands", "on", "each", "of", "your",
            "turns",
        ],
        message: "unsupported additional-land-play permission clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "target",
            "creature",
            "can",
            "block",
            "any",
            "number",
            "of",
            "creatures",
            "this",
            "turn",
        ],
        message: "unsupported target-only restriction clause",
    },
    UnsupportedWordRule {
        phrase: &["equip", "costs", "you", "pay", "cost", "1", "less"],
        message: "unsupported activation cost modifier clause",
    },
    UnsupportedWordRule {
        phrase: &["unleash", "while"],
        message: "unsupported line",
    },
];

struct UnsupportedRewriteLineContext {
    words: Vec<String>,
}

impl UnsupportedRewriteLineContext {
    fn new(tokens: &[OwnedLexToken]) -> Self {
        Self {
            words: TokenWordView::new(tokens).owned_words(),
        }
    }

    fn has_prefix(&self, expected: &[&str]) -> bool {
        self.words.len() >= expected.len()
            && self
                .words
                .iter()
                .take(expected.len())
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected)
    }

    fn contains_phrase(&self, expected: &[&str]) -> bool {
        self.phrase_count(expected) > 0
    }

    fn phrase_count(&self, expected: &[&str]) -> usize {
        if expected.is_empty() || self.words.len() < expected.len() {
            return 0;
        }

        let mut count = 0usize;
        let last_start = self.words.len() - expected.len();
        let mut start = 0usize;
        while start <= last_start {
            let matches = self.words[start..start + expected.len()]
                .iter()
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected);
            if matches {
                count += 1;
            }
            start += 1;
        }
        count
    }

    fn equals_words(&self, expected: &[&str]) -> bool {
        self.words.len() == expected.len() && self.has_prefix(expected)
    }

    fn contains_word(&self, expected: &str) -> bool {
        self.words.iter().any(|word| word == expected)
    }

    fn first_word(&self) -> Option<&str> {
        self.words.first().map(String::as_str)
    }
}

pub(super) fn diagnose_known_unsupported_rewrite_line(
    tokens: &[OwnedLexToken],
) -> Option<CardTextError> {
    let ctx = UnsupportedRewriteLineContext::new(tokens);

    for rule in UNSUPPORTED_STARTS_WITH_RULES {
        if ctx.has_prefix(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    for rule in UNSUPPORTED_EQUALS_RULES {
        if ctx.equals_words(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    for rule in UNSUPPORTED_CONTAINS_RULES {
        if ctx.contains_phrase(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    let message = if ctx.has_prefix(&["choose", "target", "land"])
        && ctx.contains_phrase(&[
            "create", "three", "tokens", "that", "are", "copies", "of", "it",
        ]) {
        "unsupported choose-leading spell clause"
    } else if ctx.contains_phrase(&["loses", "all", "abilities", "and", "becomes"]) {
        if ctx.has_prefix(&["until", "end", "of", "turn"]) {
            "unsupported loses-all-abilities with becomes clause"
        } else {
            "unsupported lose-all-abilities static becomes clause"
        }
    } else if ctx.phrase_count(&["spent", "to", "cast", "this", "spell"]) >= 2
        && ctx.contains_word("if")
        && !matches!(ctx.first_word(), Some("if" | "unless" | "when" | "as"))
    {
        "unsupported spent-to-cast conditional clause"
    } else if ctx.contains_phrase(&["for", "each", "odd", "result"])
        && ctx.contains_phrase(&["for", "each", "even", "result"])
    {
        "unsupported odd-or-even die-result clause"
    } else if ctx.contains_phrase(&[
        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its", "owner", "may",
        "play", "it",
    ]) && !ctx.contains_phrase(&[
        "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
    ]) && !ctx.contains_phrase(&["a", "spell", "cast", "this", "way", "costs"])
    {
        "unsupported for-as-long-as play/cast permission clause"
    } else if ctx.contains_phrase(&[
        "each",
        "player",
        "loses",
        "x",
        "life",
        "discards",
        "x",
        "cards",
        "sacrifices",
        "x",
        "creatures",
    ]) && ctx.contains_phrase(&["then", "sacrifices", "x", "lands"])
    {
        "unsupported multi-step each-player clause with 'then'"
    } else if ctx.has_prefix(&["target", "artifact", "creature", "or", "player"]) {
        "unsupported target artifact-creature-or-player clause"
    } else if ctx.has_prefix(&[
        "target",
        "creature",
        "token",
        "player",
        "or",
        "planeswalker",
    ]) {
        "unsupported creature-token/player/planeswalker target clause"
    } else if ctx.first_word() == Some("villainous") {
        "unsupported villainous-choice clause"
    } else if ctx.has_prefix(&["copy", "target", "spell"]) && ctx.contains_word("legendary") {
        "unsupported copy-spell legendary-exception clause"
    } else {
        return None;
    };

    Some(CardTextError::ParseError(message.to_string()))
}
