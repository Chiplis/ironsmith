use std::ops::Range;

use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreationWordClass {
    And,
    Article,
    ArticleOrThe,
    Attacking,
    Cast,
    Copy,
    Counter,
    Create,
    Decayed,
    DescriptorEnd,
    Except,
    GrantVerb,
    Half,
    Haste,
    InlineReference,
    Legendary,
    LoseVerb,
    Named,
    Of,
    On,
    Once,
    OpponentReference,
    PreserveRulesTail,
    Power,
    Put,
    RulesTextStart,
    Soulbond,
    SourceCounterLeading,
    Spell,
    Tapped,
    This,
    Time,
    Token,
    Toughness,
    Turn,
    Twice,
    When,
    Where,
    With,
    YouReference,
}

impl CreationWordClass {
    fn words(self) -> &'static [&'static str] {
        match self {
            Self::And => &["and"],
            Self::Article => &["a", "an"],
            Self::ArticleOrThe => &["a", "an", "the"],
            Self::Attacking => &["attacking"],
            Self::Cast => &["cast", "casts"],
            Self::Copy => &["copy", "copies"],
            Self::Counter => &["counter", "counters"],
            Self::Create => &["create", "creates"],
            Self::Decayed => &["decayed"],
            Self::DescriptorEnd => &["with", "has", "have", "gain", "gains"],
            Self::Except => &["except"],
            Self::GrantVerb => &["has", "have", "gain", "gains"],
            Self::Half => &["half"],
            Self::Haste => &["haste"],
            Self::InlineReference => &["that", "it", "those", "thats", "its"],
            Self::Legendary => &["legendary"],
            Self::LoseVerb => &["lose", "loses"],
            Self::Named => &["named"],
            Self::Of => &["of"],
            Self::On => &["on"],
            Self::Once => &["once"],
            Self::OpponentReference => &["opponent", "opponents"],
            Self::PreserveRulesTail => &[
                "when",
                "whenever",
                "at",
                "sacrifice",
                "return",
                "counter",
                "draw",
                "add",
                "deals",
                "deal",
                "gets",
                "gain",
                "gains",
                "power",
                "toughness",
                "cant",
                "can",
                "block",
            ],
            Self::Power => &["power"],
            Self::Put => &["put"],
            Self::RulesTextStart => &[
                "when",
                "whenever",
                "if",
                "t",
                "this",
                "that",
                "it",
                "those",
                "sacrifice",
                "add",
                "draw",
                "deals",
                "deal",
            ],
            Self::Soulbond => &["soulbond"],
            Self::SourceCounterLeading => &["a", "an", "one", "another"],
            Self::Spell => &["spell", "spells"],
            Self::Tapped => &["tapped"],
            Self::This => &["this"],
            Self::Time => &["time", "times"],
            Self::Token => &["token", "tokens"],
            Self::Toughness => &["toughness"],
            Self::Turn => &["turn"],
            Self::Twice => &["twice"],
            Self::When => &["when", "whenever"],
            Self::Where => &["where"],
            Self::With => &["with"],
            Self::YouReference => &["you", "your", "youve"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreationPhrase {
    AbilityFromAmong,
    AdditionToOtherTypes,
    AtEndOfCombat,
    AttachedTo,
    AttackingThatPlayer,
    AttackTarget,
    BasicLandTypes,
    CardExiledThisWay,
    CardTypesAmong,
    ColorsOfMana,
    CreatureDiedThisTurn,
    EqualTo,
    EquipmentRules,
    ExiledThisWay,
    ForEach,
    FoundAmong,
    GetsForEach,
    GraveyardOrHandThisWay,
    HasteGrant,
    IdentityClause,
    InlineModifierStart,
    InlineRulesTail,
    NotLegendary,
    NumberOf,
    ObjectExiledThisWay,
    OtherThanFirst,
    RegeneratedThisTurn,
    SourceCounterReference,
    ThisWay,
    TokenRulesText,
    TrailingNextEndStep,
    Unblockable,
    WithFlying,
    WithTrample,
    YouControl,
}

impl CreationPhrase {
    fn alternatives(self) -> &'static [&'static [&'static str]] {
        match self {
            Self::AbilityFromAmong => &[
                &["ability", "from", "among"],
                &["abilities", "from", "among"],
            ],
            Self::AdditionToOtherTypes => &[
                &["in", "addition", "to", "its", "other", "creature", "types"],
                &[
                    "in", "addition", "to", "their", "other", "creature", "types",
                ],
                &["in", "addition", "to", "its", "other", "types"],
                &["in", "addition", "to", "their", "other", "types"],
            ],
            Self::AtEndOfCombat => &[&["at", "end", "of", "combat"]],
            Self::AttachedTo => &[&["attached", "to"]],
            Self::AttackingThatPlayer => &[&["attacking", "that", "player"]],
            Self::AttackTarget => &[
                &[
                    "that",
                    "player",
                    "or",
                    "a",
                    "planeswalker",
                    "they",
                    "control",
                ],
                &["that", "player", "or", "planeswalker", "they", "control"],
                &[
                    "that",
                    "player",
                    "or",
                    "a",
                    "planeswalker",
                    "they",
                    "controls",
                ],
                &["that", "player", "or", "planeswalker", "they", "controls"],
                &[
                    "that",
                    "player",
                    "or",
                    "a",
                    "planeswalker",
                    "their",
                    "control",
                ],
                &["that", "player", "or", "planeswalker", "their", "control"],
            ],
            Self::BasicLandTypes => &[
                &["basic", "land", "type", "among", "lands", "you", "control"],
                &["basic", "land", "types", "among", "lands", "you", "control"],
                &[
                    "basic", "land", "type", "among", "the", "lands", "you", "control",
                ],
                &[
                    "basic", "land", "types", "among", "the", "lands", "you", "control",
                ],
            ],
            Self::CardExiledThisWay => &[
                &["card", "exiled", "this", "way"],
                &["cards", "exiled", "this", "way"],
            ],
            Self::CardTypesAmong => &[&["card", "type", "among"], &["card", "types", "among"]],
            Self::ColorsOfMana => &[
                &[
                    "color", "of", "mana", "spent", "to", "cast", "this", "spell",
                ],
                &[
                    "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
                ],
                &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
                &[
                    "colors", "of", "mana", "used", "to", "cast", "this", "spell",
                ],
            ],
            Self::CreatureDiedThisTurn => &[
                &["creature", "that", "died", "this", "turn"],
                &["creatures", "that", "died", "this", "turn"],
            ],
            Self::EqualTo => &[&["equal", "to"]],
            Self::EquipmentRules => &[
                &["equipped", "creature", "has"],
                &["equipped", "creature", "gets"],
            ],
            Self::ExiledThisWay => &[&["exiled", "this", "way"]],
            Self::ForEach => &[&["for", "each"]],
            Self::FoundAmong => &[&["found", "among"]],
            Self::GetsForEach => &[
                &["this", "token", "gets", "+1/+1", "for", "each"],
                &["this", "creature", "gets", "+1/+1", "for", "each"],
            ],
            Self::GraveyardOrHandThisWay => &[
                &["card", "put", "into", "a", "graveyard", "this", "way"],
                &["cards", "put", "into", "a", "graveyard", "this", "way"],
                &["object", "put", "into", "a", "graveyard", "this", "way"],
                &["objects", "put", "into", "a", "graveyard", "this", "way"],
                &["permanent", "put", "into", "a", "graveyard", "this", "way"],
                &["permanents", "put", "into", "a", "graveyard", "this", "way"],
                &["card", "put", "into", "graveyard", "this", "way"],
                &["cards", "put", "into", "graveyard", "this", "way"],
                &["object", "put", "into", "graveyard", "this", "way"],
                &["objects", "put", "into", "graveyard", "this", "way"],
                &["permanent", "put", "into", "graveyard", "this", "way"],
                &["permanents", "put", "into", "graveyard", "this", "way"],
                &["card", "exiled", "from", "their", "hand", "this", "way"],
                &["cards", "exiled", "from", "their", "hand", "this", "way"],
                &[
                    "card", "exiled", "from", "his", "or", "her", "hand", "this", "way",
                ],
                &[
                    "cards", "exiled", "from", "his", "or", "her", "hand", "this", "way",
                ],
            ],
            Self::HasteGrant => &[
                &["has", "haste"],
                &["gain", "haste"],
                &["gains", "haste"],
                &["that", "token", "gains", "haste"],
                &["those", "tokens", "gain", "haste"],
            ],
            Self::IdentityClause => &[
                &["its"],
                &["it", "is"],
                &["it", "s"],
                &["it's"],
                &["it’s"],
                &["that", "copy", "is"],
                &["that", "the", "copy", "is"],
                &["theyre"],
                &["they", "re"],
                &["they're"],
                &["they’re"],
                &["they", "are"],
            ],
            Self::InlineModifierStart => &[
                &["thats"],
                &["that's"],
                &["that’s"],
                &["that", "s"],
                &["that", "is"],
                &["that", "are"],
            ],
            Self::InlineRulesTail => &[
                &["when"],
                &["whenever"],
                &["this", "token"],
                &["that", "token"],
                &["those", "tokens"],
                &["except", "it"],
                &["except", "they"],
                &["except", "its"],
                &["except", "their"],
                &["this", "creature"],
                &["that", "creature"],
                &["at", "the", "beginning"],
                &["at", "beginning"],
                &["sacrifice", "this", "token"],
                &["sacrifice", "that", "token"],
                &["sacrifice", "this", "permanent"],
                &["sacrifice", "that", "permanent"],
                &["sacrifice", "it"],
                &["sacrifice", "them"],
                &["it", "has"],
                &["it", "gains"],
                &["they", "have"],
                &["they", "gain"],
                &["equip"],
                &["equipped", "creature"],
                &["enchanted", "creature"],
                &["r"],
                &["t"],
            ],
            Self::NotLegendary => &[
                &["isnt", "legendary"],
                &["isn't", "legendary"],
                &["is", "not", "legendary"],
                &["its", "not", "legendary"],
                &["it's", "not", "legendary"],
                &["it", "s", "not", "legendary"],
            ],
            Self::NumberOf => &[&["a", "number", "of"], &["the", "number", "of"]],
            Self::ObjectExiledThisWay => &[
                &["object", "exiled", "this", "way"],
                &["objects", "exiled", "this", "way"],
                &["permanent", "exiled", "this", "way"],
                &["permanents", "exiled", "this", "way"],
            ],
            Self::OtherThanFirst => &[&["other", "than", "the", "first"]],
            Self::RegeneratedThisTurn => &[
                &["time", "it", "regenerated", "this", "turn"],
                &["times", "it", "regenerated", "this", "turn"],
            ],
            Self::SourceCounterReference => &[
                &["it"],
                &["this"],
                &["this", "card"],
                &["this", "creature"],
                &["this", "permanent"],
                &["this", "source"],
                &["this", "artifact"],
                &["this", "land"],
                &["this", "enchantment"],
            ],
            Self::ThisWay => &[&["this", "way"]],
            Self::TokenRulesText => &[
                &["it", "has"],
                &["it", "gains"],
                &["it", "gets"],
                &["this", "token"],
                &["that", "token"],
            ],
            Self::TrailingNextEndStep => &[
                &[
                    "at",
                    "the",
                    "beginning",
                    "of",
                    "your",
                    "next",
                    "end",
                    "step",
                ],
                &["at", "the", "beginning", "of", "the", "next", "end", "step"],
                &["at", "the", "beginning", "of", "next", "end", "step"],
                &["at", "the", "beginning", "of", "the", "end", "step"],
                &["at", "the", "beginning", "of", "end", "step"],
            ],
            Self::Unblockable => &[
                &["this", "token", "cant", "be", "blocked"],
                &["this", "creature", "cant", "be", "blocked"],
                &["cant", "be", "blocked"],
            ],
            Self::WithFlying => &[&["with", "flying"]],
            Self::WithTrample => &[&["with", "trample"]],
            Self::YouControl => &[&["you", "control"]],
        }
    }
}

fn parse_dynamic_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("creation phrase", "word"));
        };
        if *word != *expected_word {
            return Err(primitives::backtrack_err("creation phrase", "word"));
        }
        *input = rest;
    }
    Ok(())
}

fn phrase_at_start(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_dynamic_phrase(&mut input, phrase).is_ok()
}

fn phrase_at_end(words: &[&str], phrase: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(
        0..,
        any.void(),
        (
            |candidate: &mut primitives::WordSliceInput<'_>| {
                parse_dynamic_phrase(candidate, phrase)
            },
            eof,
        )
            .void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    Some(skipped.len())
}

fn phrase_anywhere(words: &[&str], phrase: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| {
            parse_dynamic_phrase(candidate, phrase)
        }),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    Some(skipped.len())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CreationWords<'a> {
    words: &'a [&'a str],
}

impl<'a> CreationWords<'a> {
    pub(crate) fn new(words: &'a [&'a str]) -> Self {
        Self { words }
    }

    pub(crate) fn class_at(self, idx: usize, class: CreationWordClass) -> bool {
        self.words
            .get(idx)
            .is_some_and(|word| class.words().iter().any(|expected| word == expected))
    }

    pub(crate) fn first_is(self, class: CreationWordClass) -> bool {
        self.class_at(0, class)
    }

    pub(crate) fn has(self, class: CreationWordClass) -> bool {
        class
            .words()
            .iter()
            .any(|expected| phrase_anywhere(self.words, &[*expected]).is_some())
    }

    pub(super) fn has_literal(self, expected: &str) -> bool {
        self.words.contains(&expected)
    }

    pub(crate) fn location(self, class: CreationWordClass) -> Option<usize> {
        class
            .words()
            .iter()
            .filter_map(|expected| phrase_anywhere(self.words, &[*expected]))
            .min()
    }

    pub(crate) fn starts(self, phrase: CreationPhrase) -> bool {
        phrase
            .alternatives()
            .iter()
            .any(|expected| phrase_at_start(self.words, expected))
    }

    pub(crate) fn has_phrase(self, phrase: CreationPhrase) -> bool {
        self.phrase_location(phrase).is_some()
    }

    pub(crate) fn phrase_location(self, phrase: CreationPhrase) -> Option<usize> {
        phrase
            .alternatives()
            .iter()
            .filter_map(|expected| phrase_anywhere(self.words, expected))
            .min()
    }

    pub(crate) fn exact(self, phrase: CreationPhrase) -> bool {
        phrase.alternatives().iter().any(|expected| {
            expected.len() == self.words.len() && phrase_at_start(self.words, expected)
        })
    }

    pub(crate) fn suffix_location(self, phrase: CreationPhrase) -> Option<usize> {
        phrase
            .alternatives()
            .iter()
            .filter_map(|expected| phrase_at_end(self.words, expected))
            .min()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CreationTokens<'a> {
    tokens: &'a [OwnedLexToken],
}

impl<'a> CreationTokens<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens }
    }

    pub(crate) fn words(self) -> Vec<&'a str> {
        TokenWordView::new(self.tokens).word_refs()
    }

    pub(crate) fn boundary(self, word_idx: usize) -> Option<usize> {
        let view = TokenWordView::new(self.tokens);
        if word_idx == view.len() {
            return Some(self.tokens.len());
        }
        view.token_span_for_words(word_idx, word_idx + 1)
            .map(|range| range.start)
    }

    pub(crate) fn word_range(self, words: Range<usize>) -> Option<&'a [OwnedLexToken]> {
        let view = TokenWordView::new(self.tokens);
        let range = view.token_span_for_words(words.start, words.end)?;
        Some(&self.tokens[range])
    }

    pub(crate) fn token_is(self, token_idx: usize, class: CreationWordClass) -> bool {
        self.tokens.get(token_idx).is_some_and(|token| {
            let text = token.parser_text();
            class.words().contains(&text)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn typed_surface_finds_creation_vocabulary() {
        let tokens = lex_line(
            "a token that's tapped and attacking that player at the beginning of the next end step",
            0,
        )
        .unwrap();
        let words = CreationTokens::new(&tokens).words();
        let surface = CreationWords::new(&words);
        assert!(surface.has(CreationWordClass::Tapped));
        assert!(surface.has_phrase(CreationPhrase::AttackingThatPlayer));
        assert!(
            surface
                .suffix_location(CreationPhrase::TrailingNextEndStep)
                .is_some()
        );
    }

    #[test]
    fn typed_surface_words_share_coordinates_with_token_boundaries() {
        let tokens =
            lex_line("a 2/2 colorless Assembly-Worker artifact creature token", 0).unwrap();
        let surface = CreationTokens::new(&tokens);
        let words = surface.words();
        assert_eq!(
            words,
            [
                "a",
                "2/2",
                "colorless",
                "assembly",
                "worker",
                "artifact",
                "creature",
                "token",
            ]
        );
        let token_word = CreationWords::new(&words)
            .location(CreationWordClass::Token)
            .expect("token marker");
        let marker_token = surface.boundary(token_word).expect("marker boundary");
        assert!(tokens[marker_token].is_word("token"));
    }
}
