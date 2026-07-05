use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate, OwnedLexToken,
    PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    RedirectNextTimeDamageDestinationAst, SubjectAst, SubjectVerbActionAst, TagKey, TargetAst,
    TextSpan, Verb,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use crate::{ChoiceCount, Supertype};

use super::super::activation_and_restrictions::activation_restriction_clauses::starts_with_target_indicator;
use super::super::activation_and_restrictions::trigger_subject_filters::title_case_token_word;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexToken, LexedClause, find_token_word_sequence, token_slice_at_is, token_slice_last_is,
    token_slice_starts_with, word_slice_contains_word, word_slice_ends_with, word_slice_eq,
    word_slice_eq_any, word_slice_find_phrase_start, word_slice_find_word_where,
    word_slice_starts_with,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index as find_token_index, rfind_index as find_token_index_rev,
};
use super::super::util::{
    non_article_token_word_refs, parse_card_type, parse_choice_count_before_target_prefix,
    parse_choice_count_token_prefix_consumed, parse_color, parse_counter_type_from_tokens,
    parse_counter_type_word, parse_number, parse_subject, parse_target_phrase, parse_value,
    span_from_tokens, strip_leading_article_word_refs, token_index_for_word_index, trim_commas,
    wrap_target_count,
};
use super::chain_carry::find_verb;
use super::parse_subtype_word;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, parse_distribute_counters_sentence,
};
use super::verb_dispatch::parse_effect_with_verb;

type ClausePatternCompatWords<'a> = TokenWordView<'a>;

const DAMAGE_THAT_WOULD_BE_DEALT_TO_WORDS: &[&str] =
    &["damage", "that", "would", "be", "dealt", "to"];
const CLAUSE_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    ClauseShape::new().exact_any(&[&["and"], &["or"]]);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClauseShape<'p> {
    exact: Option<&'p [&'p str]>,
    exact_any: &'p [&'p [&'p str]],
    prefix: Option<&'p [&'p str]>,
    prefix_any: &'p [&'p [&'p str]],
    suffix: Option<&'p [&'p str]>,
    suffix_any: &'p [&'p [&'p str]],
    contains_phrases: &'p [&'p [&'p str]],
    contains_any_phrases: &'p [&'p [&'p [&'p str]]],
    contains_words: &'p [&'p str],
    contains_any_words: &'p [&'p [&'p str]],
}

#[allow(dead_code)]
impl<'p> ClauseShape<'p> {
    pub(crate) const fn new() -> Self {
        Self {
            exact: None,
            exact_any: &[],
            prefix: None,
            prefix_any: &[],
            suffix: None,
            suffix_any: &[],
            contains_phrases: &[],
            contains_any_phrases: &[],
            contains_words: &[],
            contains_any_words: &[],
        }
    }

    pub(crate) const fn exact(mut self, phrase: &'p [&'p str]) -> Self {
        self.exact = Some(phrase);
        self
    }

    pub(crate) const fn exact_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.exact_any = phrases;
        self
    }

    pub(crate) const fn prefix(mut self, phrase: &'p [&'p str]) -> Self {
        self.prefix = Some(phrase);
        self
    }

    pub(crate) const fn prefix_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.prefix_any = phrases;
        self
    }

    pub(crate) const fn suffix(mut self, phrase: &'p [&'p str]) -> Self {
        self.suffix = Some(phrase);
        self
    }

    pub(crate) const fn suffix_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.suffix_any = phrases;
        self
    }

    pub(crate) const fn contains_phrases(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.contains_phrases = phrases;
        self
    }

    pub(crate) const fn contains_any_phrases(mut self, phrases: &'p [&'p [&'p [&'p str]]]) -> Self {
        self.contains_any_phrases = phrases;
        self
    }

    pub(crate) const fn contains_words(mut self, words: &'p [&'p str]) -> Self {
        self.contains_words = words;
        self
    }

    pub(crate) const fn contains_any_words(mut self, word_sets: &'p [&'p [&'p str]]) -> Self {
        self.contains_any_words = word_sets;
        self
    }

    pub(crate) fn matches(self, clause: LexedClause<'_>) -> bool {
        if let Some(exact) = self.exact
            && !clause.matches_words(exact)
        {
            return false;
        }
        if !self.exact_any.is_empty() && !clause.matches_any_words(self.exact_any) {
            return false;
        }
        if let Some(prefix) = self.prefix
            && !clause.starts_with(prefix)
        {
            return false;
        }
        if !self.prefix_any.is_empty() && !clause.starts_with_any(self.prefix_any) {
            return false;
        }
        if let Some(suffix) = self.suffix
            && !clause.ends_with(suffix)
        {
            return false;
        }
        if !self.suffix_any.is_empty() && !clause.ends_with_any(self.suffix_any) {
            return false;
        }
        if self
            .contains_phrases
            .iter()
            .any(|phrase| !clause.contains_phrase(phrase))
        {
            return false;
        }
        if self
            .contains_any_phrases
            .iter()
            .any(|phrases| !clause.contains_any_phrase(phrases))
        {
            return false;
        }
        if self
            .contains_words
            .iter()
            .any(|word| !clause.contains_word(word))
        {
            return false;
        }
        if self
            .contains_any_words
            .iter()
            .any(|words| !clause.contains_any_word(words))
        {
            return false;
        }
        true
    }

    pub(crate) fn matches_non_article_tokens(self, tokens: &[OwnedLexToken]) -> bool {
        self.matches_words(&non_article_token_word_refs(tokens))
    }

    pub(crate) fn matches_word(self, word: &str) -> bool {
        self.matches_words(&[word])
    }

    pub(crate) fn matches_token(self, token: &OwnedLexToken) -> bool {
        token
            .as_word()
            .is_some_and(|_| self.matches_word(token.parser_text()))
    }

    pub(crate) fn matches_lex_token(self, token: &LexToken) -> bool {
        token
            .as_word()
            .is_some_and(|_| self.matches_word(token.parser_text()))
    }

    pub(crate) fn matches_word_at(self, words: &[&str], idx: usize) -> bool {
        words.get(idx).is_some_and(|word| self.matches_word(word))
    }

    pub(crate) fn matches_first_word(self, words: &[&str]) -> bool {
        self.matches_word_at(words, 0)
    }

    pub(crate) fn matches_last_word(self, words: &[&str]) -> bool {
        words.last().is_some_and(|word| self.matches_word(word))
    }

    pub(crate) fn find_word(self, words: &[&str]) -> Option<usize> {
        words.iter().position(|word| self.matches_word(word))
    }

    pub(crate) fn rfind_word(self, words: &[&str]) -> Option<usize> {
        words.iter().rposition(|word| self.matches_word(word))
    }

    pub(crate) fn find_exact_window(self, words: &[&str], width: usize) -> Option<usize> {
        words
            .windows(width)
            .position(|window| self.matches_words(window))
    }

    pub(crate) fn find_exact_window_range(
        self,
        words: &[&str],
        min_width: usize,
        max_width: usize,
    ) -> Option<usize> {
        (min_width..=max_width).find_map(|width| self.find_exact_window(words, width))
    }

    pub(crate) fn matches_clause_first_word(self, clause: LexedClause<'_>) -> bool {
        self.matches_first_word(&clause.word_refs())
    }

    /// Token-backed word-slice gate exposed for shared-util shape helpers.
    ///
    /// Behaviorally identical to [`Self::matches_words`]; provided under a
    /// distinct name so callers outside this primitive module can route shape
    /// gates through a helper without invoking the `matches_words` adapter
    /// directly.
    pub(crate) fn matches_word_slice(self, words: &[&str]) -> bool {
        self.matches_words(words)
    }

    pub(crate) fn matches_words(self, words: &[&str]) -> bool {
        if let Some(exact) = self.exact
            && !word_slice_eq(words, exact)
        {
            return false;
        }
        if !self.exact_any.is_empty() && !word_slice_eq_any(words, self.exact_any) {
            return false;
        }
        if let Some(prefix) = self.prefix
            && !word_slice_starts_with(words, prefix)
        {
            return false;
        }
        if !self.prefix_any.is_empty()
            && !self
                .prefix_any
                .iter()
                .any(|phrase| word_slice_starts_with(words, phrase))
        {
            return false;
        }
        if let Some(suffix) = self.suffix
            && !word_slice_ends_with(words, suffix)
        {
            return false;
        }
        if !self.suffix_any.is_empty()
            && !self
                .suffix_any
                .iter()
                .any(|phrase| word_slice_ends_with(words, phrase))
        {
            return false;
        }
        if self
            .contains_phrases
            .iter()
            .any(|phrase| word_slice_find_phrase_start(words, phrase).is_none())
        {
            return false;
        }
        if self.contains_any_phrases.iter().any(|phrases| {
            !phrases
                .iter()
                .any(|phrase| word_slice_find_phrase_start(words, phrase).is_some())
        }) {
            return false;
        }
        if self
            .contains_words
            .iter()
            .any(|word| !word_slice_contains_word(words, word))
        {
            return false;
        }
        if self.contains_any_words.iter().any(|word_set| {
            !word_set
                .iter()
                .any(|word| word_slice_contains_word(words, word))
        }) {
            return false;
        }
        true
    }

    pub(crate) fn matched_prefix_len(self, words: &[&str]) -> Option<usize> {
        if let Some(prefix) = self.prefix
            && word_slice_starts_with(words, prefix)
        {
            return Some(prefix.len());
        }
        self.prefix_any
            .iter()
            .find_map(|prefix| word_slice_starts_with(words, prefix).then_some(prefix.len()))
    }
}

macro_rules! clause_shape {
    (exact $phrase:expr) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .exact($phrase)
    };
    (exact_any $phrases:expr) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .exact_any($phrases)
    };
    (prefix $prefix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix($prefix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix_any($prefixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix $prefix:expr; suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix($prefix)
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr; suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix_any($prefixes)
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr; suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix_any($prefixes)
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix $prefix:expr; suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .prefix($prefix)
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (suffix $suffix:expr) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .suffix($suffix)
    };
    (suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_phrases $contains_phrases:expr $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .contains_phrases($contains_phrases)
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_any_phrases $contains_any_phrases:expr $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .contains_any_phrases($contains_any_phrases)
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_words $contains_words:expr $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .contains_words($contains_words)
            $(.contains_any_words($contains_any_words))?
    };
    (contains_any_words $contains_any_words:expr $(; contains_words $contains_words:expr)?) => {
        $crate::runtime_backend::effect_sentences::clause_pattern_helpers::ClauseShape::new()
            .contains_any_words($contains_any_words)
            $(.contains_words($contains_words))?
    };
}

pub(crate) use clause_shape;

const CLAUSE_COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const CLAUSE_DOUBLE_NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["double", "the", "number", "of"]);
const CLAUSE_ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const CLAUSE_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const CLAUSE_AND_OR_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const CLAUSE_EACH_OR_ALL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each"], &["all"]]);
const CLAUSE_YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const CLAUSE_PREVENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["prevent"]);
const CLAUSE_THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const CLAUSE_NEXT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["next"]);
const CLAUSE_YOU_PREFIX: &[&str] = &["you"];
const CLAUSE_THAT_PLAYER_PREFIX: &[&str] = &["that", "player"];
const CLAUSE_MAY_PREFIX: &[&str] = &["may"];
const CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const CLAUSE_DONT_OR_DONT_APOSTROPHE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dont"], &["don't"]]);
const CLAUSE_DO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["do"]);
const CLAUSE_NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const CLAUSE_FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const CLAUSE_TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const CLAUSE_BY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["by"]);
const CLAUSE_TARGET_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const CLAUSE_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const CLAUSE_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "turn"]);
const CLAUSE_CAN_ATTACK_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["can", "attack"]);
const CLAUSE_CAN_BLOCK_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["can", "block"]);
const CLAUSE_AS_THOUGH_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["as", "though"]]);
const CLAUSE_TURN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["turn"]);
const CLAUSE_HAVE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["have"]);
const CLAUSE_DEFENDER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["defender"]);
const CLAUSE_DIDNT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["didnt"], &["didn't"]]);
const CLAUSE_DEAL_DAMAGE_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["deal", "damage", "to"]);
const CLAUSE_DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
const CLAUSE_PREVENT_ALL_DAMAGE_TO_PREFIX: &[&str] = &[
    "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
];
const CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_TO_PREFIX: &[&str] = &[
    "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
];
const CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_BY_PREFIX: &[&str] = &[
    "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "by",
];
const CLAUSE_PREVENT_ALL_DAMAGE_TO_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix CLAUSE_PREVENT_ALL_DAMAGE_TO_PREFIX; suffix &["this", "turn"]);
const CLAUSE_THIS_TURN_BY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "turn", "by"]);
const CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_TO_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_TO_PREFIX);
const CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_BY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_BY_PREFIX);
const CLAUSE_ALL_DAMAGE_WOULD_BE_DEALT_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["all", "damage", "that", "would", "be", "dealt", "to"]);
const CLAUSE_SOURCES_SUFFIX: &[&str] = &["sources"];
const CLAUSE_SOURCES_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["sources"]);
const CLAUSE_THE_NEXT_TIME_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "next", "time"]);
const CLAUSE_THE_NEXT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "next"]);
const CLAUSE_PREVENT_THAT_DAMAGE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["prevent", "that", "damage"]);
const CLAUSE_PREVENT_THAT_DAMAGE_IF_PREVENTED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "prevent",
            "that",
            "damage",
            "if",
            "damage",
            "is",
            "prevented",
            "this",
            "way",
        ];
    contains_phrases
        & [&[
            "deals",
            "that",
            "much",
            "damage",
            "to",
            "that",
            "source's",
            "controller",
        ]]
);
const CLAUSE_THAT_DAMAGE_IS_DEALT_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "damage", "is", "dealt", "to"]);
const CLAUSE_THAT_SOURCE_DEALS_THAT_DAMAGE_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "source", "deals", "that", "damage", "to"]);
const CLAUSE_IS_DEALT_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["is", "dealt", "to"]);
const CLAUSE_INSTEAD_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["instead"]);
const CLAUSE_SOURCE_OF_YOUR_CHOICE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["of", "your", "choice"]]);
const CLAUSE_YOU_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const CLAUSE_ANY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["any", "target"]);
const CLAUSE_SOURCE_OR_SOURCES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["source"], &["sources"]]);
const CLAUSE_SOURCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["source"]);
const CLAUSE_THAT_WOULD_BE_DEALT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "would", "be", "dealt"]);
const CLAUSE_REDIRECT_DAMAGE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to"
        ]
);
const CLAUSE_SHADOW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["shadow"]);
const CLAUSE_CREATURE_OR_CREATURES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature"], &["creatures"]]);
const CLAUSE_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const CLAUSE_YOU_WIN_GAME_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "win", "the", "game"]);
const CLAUSE_IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const CLAUSE_OWN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["own"]);
const CLAUSE_CARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["card"]);
const CLAUSE_NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const CLAUSE_IN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["in"]);
const CLAUSE_INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const CLAUSE_AMASS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["amass"]);
const CLAUSE_FORAGE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["forage"], &["forages"]]);
const CLAUSE_ROLL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["roll"]);
const CLAUSE_DICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["dice"]);
const CLAUSE_DICE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["dice"]);
const CLAUSE_SIX_SIDED_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["six", "sided"]);
const CLAUSE_SIMULTANEOUSLY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["simultaneously"]);
const CLAUSE_ALL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["all"]);
const CLAUSE_PHASED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["phased-out"], &["phased"]]);
const CLAUSE_BEHOLD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["behold"]);
const CLAUSE_BLIGHT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["blight"]);
const CLAUSE_UNSUPPORTED_KEYWORD_EFFECT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dredge"], &["warp"]]);
const CLAUSE_HARNESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["harness"]);
const CLAUSE_MANIFEST_DREAD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["manifest", "dread"]);
const CLAUSE_TWICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["twice"]);
const CLAUSE_TIME_OR_TIMES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["time"], &["times"]]);
const CLAUSE_MANIFEST_TOP_YOUR_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["manifest", "the", "top", "card", "of", "your", "library"]);
const CLAUSE_MANIFEST_CARD_FROM_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["manifest", "a", "card", "from", "your", "hand"]);
const CLAUSE_MANIFEST_TOP_THAT_PLAYER_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "manifest", "the", "top", "card", "of", "that", "player's", "library"
            ],
            &[
                "manifest", "the", "top", "card", "of", "that", "players", "library"
            ],
        ]
);
const CLAUSE_ITS_CONTROLLER_MANIFESTS_TOP_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "its",
                "controller",
                "manifests",
                "the",
                "top",
                "card",
                "of",
                "their",
                "library"
            ],
            &[
                "that",
                "player",
                "manifests",
                "the",
                "top",
                "card",
                "of",
                "their",
                "library"
            ],
        ]
);
const CLAUSE_POPULATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["populate"]);
const CLAUSE_MELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["meld"]);
const CLAUSE_MELD_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["them"], &["those", "cards"]]);
const CLAUSE_BOLSTER_SUPPORT_ADAPT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["bolster"], &["support"], &["adapt"]]);
const CLAUSE_FATESEAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["fateseal"]);
const CLAUSE_DISCOVER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["discover"], &["discovers"]]);
const CLAUSE_DISCOVER_AGAIN_SAME_VALUE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["again", "for", "the", "same", "value"]);
const CLAUSE_EXPLORE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["explore"], &["explores"]]);
const CLAUSE_AGAIN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["again"]);
const CLAUSE_SOURCE_SUBJECT_WORDS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
        ]
);
const CLAUSE_CONNIVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["connive"], &["connives"]]);
const CLAUSE_CONVOKED_THIS_SPELL_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["each", "creature", "that", "convoked", "this", "spell"]);
const CLAUSE_EACH_OF_X_TARGET_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["each", "of", "x", "target"],
            &["each", "of", "X", "target"]
        ]
);
const CLAUSE_COPY_OR_COPIES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["copy"], &["copies"]]);
const CLAUSE_THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const CLAUSE_CHOOSE_TARGET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["choose", "target"]);
const CLAUSE_CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const CLAUSE_THEN_OR_YOU_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["then"], &["you"]]);
const CLAUSE_EXCEPT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["except"]);
const CLAUSE_SOURCE_OF_YOUR_CHOICE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["by", "a", "source", "of", "your", "choice"]);
const CLAUSE_SIMPLE_CHOSEN_OBJECT_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const CLAUSE_COPY_REFERENCE_HEAD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["this"], &["that"]]);
const CLAUSE_SIMPLE_COPY_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["copy", "that", "card"],
            &["copy", "the", "exiled", "card"]
        ]
);
const CLAUSE_THIS_SPELL_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "spell"]);
const CLAUSE_THAT_SPELL_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "spell"]);
const CLAUSE_THAT_ABILITY_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "ability"]);
const CLAUSE_THAT_SPELL_OR_ABILITY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "spell", "or", "ability"],
            &["that", "ability", "or", "spell"],
        ]
);
const CLAUSE_TAGGED_COPY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["that"],
            &["that", "card"],
            &["the", "exiled", "card"],
        ]
);
const CLAUSE_SPELL_OR_ABILITY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells", "ability", "abilities"]]);

const ODD_EVEN_RESULT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "odd", "result"],
    &["for", "each", "even", "result"],
];

const ODD_RESULT_VALUES_D6: &[i32] = &[1, 3, 5];
const EVEN_RESULT_VALUES_D6: &[i32] = &[2, 4, 6];
const OPEN_ATTRACTION_PREFIXES: &[&[&str]] = &[
    &["open", "an", "attraction"],
    &["opens", "an", "attraction"],
];

fn strip_suffix_char<'a>(word: &'a str, suffix: char) -> Option<&'a str> {
    crate::string_primitives::strip_suffix_char(word, suffix)
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    crate::slice_primitives::push_unique(items, item);
}

pub(crate) fn extract_subject_player(subject: Option<SubjectAst>) -> Option<PlayerAst> {
    match subject {
        Some(SubjectAst::Player(player)) => Some(player),
        _ => None,
    }
}

pub(crate) fn parse_prevent_next_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if !CLAUSE_PREVENT_WORD_PATTERN.matches_first_word(&clause_words) {
        return Ok(None);
    }

    let mut idx = 1usize;
    if clause_words
        .get(idx)
        .is_some_and(|word| CLAUSE_THE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if !clause_words
        .get(idx)
        .is_some_and(|word| CLAUSE_NEXT_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    idx += 1;

    let amount_token = OwnedLexToken::word(
        clause_words
            .get(idx)
            .copied()
            .unwrap_or_default()
            .to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_text
        )));
    };
    idx += amount_used;

    if !clause_words
        .get(idx)
        .is_some_and(|word| CLAUSE_DAMAGE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    idx += 1;

    if !CLAUSE_THAT_WOULD_BE_DEALT_PREFIX_PATTERN.matches_words(&clause_words[idx..]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_text
        )));
    }
    idx += 4;

    if !clause_words
        .get(idx)
        .is_some_and(|word| CLAUSE_TO_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_text
        )));
    }
    idx += 1;

    let this_turn_rel = CLAUSE_THIS_TURN_PATTERN
        .find_exact_window(&clause_words[idx..], 2)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_text
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let source_of_your_choice = if this_turn_idx + 2 == clause_words.len() {
        false
    } else if CLAUSE_SOURCE_OF_YOUR_CHOICE_TAIL_PATTERN
        .matches_words(&clause_words[this_turn_idx + 2..])
    {
        true
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_text
        )));
    };

    let target_clause = clause.between_words_trimmed(idx, this_turn_idx);
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_text
        )));
    }
    let target_words = target_clause.word_refs();
    let protects_you_and_permanents_you_control = matches!(
        target_words.as_slice(),
        ["you", "and/or", "permanents", "you", "control"]
            | ["you", "and/or", "permanent", "you", "control"]
            | ["you", "and", "or", "permanents", "you", "control"]
            | ["you", "and", "or", "permanent", "you", "control"]
            | ["you", "and", "permanents", "you", "control"]
            | ["you", "and", "permanent", "you", "control"]
    );
    let target = if protects_you_and_permanents_you_control {
        TargetAst::Player(PlayerFilter::You, span_from_tokens(target_clause.tokens()))
    } else {
        parse_target_phrase(target_clause.tokens())?
    };

    Ok(Some(EffectAst::subject_verb_prevent_damage_with_options(
        amount,
        target,
        Until::EndOfTurn,
        source_of_your_choice,
        protects_you_and_permanents_you_control,
        Vec::new(),
    )))
}

pub(crate) fn parse_double_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !CLAUSE_DOUBLE_NUMBER_OF_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let counters_idx = find_token_index(tokens, |token| {
        CLAUSE_COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counters keyword (clause: '{}')",
            clause_text
        ))
    })?;
    if counters_idx <= 4 {
        return Err(CardTextError::ParseError(format!(
            "missing counter type (clause: '{}')",
            clause_text
        )));
    }

    let counter_tokens = &tokens[4..counters_idx];
    let counter_words = crate::runtime_backend::token_word_refs(counter_tokens);
    let counter_type = if matches!(counter_words.as_slice(), ["each", "kind", "of"]) {
        None
    } else {
        Some(
            parse_counter_type_from_tokens(counter_tokens)
                .or_else(|| {
                    if counter_tokens.len() == 1 {
                        counter_tokens[0]
                            .as_word()
                            .and_then(parse_counter_type_word)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counter type in double-counters clause (clause: '{}')",
                        clause_text
                    ))
                })?,
        )
    };

    let counter_holder_words = crate::runtime_backend::token_word_refs(&tokens[counters_idx + 1..]);
    if matches!(counter_holder_words.as_slice(), ["you", "have"]) {
        return Ok(Some(EffectAst::subject_verb_double_counters_on_target(
            counter_type,
            TargetAst::Player(
                PlayerFilter::You,
                span_from_tokens(&tokens[counters_idx + 1..]),
            ),
        )));
    }

    let on_idx = find_token_index(&tokens[counters_idx + 1..], |token| {
        CLAUSE_ON_WORD_PATTERN.matches_token(token)
    })
    .map(|offset| counters_idx + 1 + offset)
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing 'on' in double-counters clause (clause: '{}')",
            clause_text
        ))
    })?;

    let mut filter_clause = clause.from(on_idx + 1).trimmed();
    if CLAUSE_EACH_OR_ALL_WORD_PATTERN.matches_clause_first_word(filter_clause) {
        filter_clause = filter_clause.from(1).trimmed();
    }
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing filter in double-counters clause (clause: '{}')",
            clause_text
        )));
    }

    let filter_words = crate::runtime_backend::token_word_refs(filter_clause.tokens());
    if matches!(
        filter_words.as_slice(),
        ["it"] | ["this"] | ["this", "creature"] | ["this", "permanent"]
    ) {
        return Ok(Some(EffectAst::subject_verb_double_counters_on_target(
            counter_type,
            TargetAst::Source(span_from_tokens(filter_clause.tokens())),
        )));
    }

    if crate::runtime_backend::token_word_refs(filter_clause.tokens())
        .iter()
        .any(|word| *word == "target" || *word == "targets")
    {
        let target = parse_target_phrase(filter_clause.tokens())?;
        return Ok(Some(EffectAst::subject_verb_double_counters_on_target(
            counter_type,
            target,
        )));
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    Ok(Some(EffectAst::subject_verb_double_counters_on_each(
        counter_type,
        filter,
    )))
}

pub(crate) fn parse_distribute_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_verb_first_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(word) = tokens.first().and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };

    let verb = match word {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "attach" => Verb::Attach,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "convert" => Verb::Convert,
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "incubate" => Verb::Incubate,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "detain" => Verb::Detain,
        "goad" => Verb::Goad,
        "suspect" => Verb::Suspect,
        "note" => Verb::Note,
        "look" => Verb::Look,
        "end" => Verb::End,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

pub(crate) fn is_simple_chosen_object_reference(tokens: &[OwnedLexToken]) -> bool {
    let raw_words = LexedClause::new(tokens).word_refs();
    let words = super::super::util::non_article_word_refs_except(&raw_words, &["then"]);
    if words.is_empty() {
        return false;
    }
    if CLAUSE_SIMPLE_CHOSEN_OBJECT_REFERENCE_PATTERN.matches_words(&words) {
        return true;
    }
    if super::for_each_helpers::has_demonstrative_object_reference(&words) {
        return true;
    }
    false
}

pub(crate) fn parse_choose_target_and_verb_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !CLAUSE_CHOOSE_TARGET_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some((before_and, after_and)) = grammar::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        grammar::kw("and").void()
    }) else {
        return Ok(None);
    };

    let target_clause = LexedClause::new(&before_and[1..]).trimmed();
    let target_tokens = target_clause.tokens();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after choose clause (clause: '{}')",
            clause_text
        )));
    }
    if find_verb(target_tokens).is_some() {
        return Ok(None);
    }

    let mut tail_clause = LexedClause::new(after_and).trimmed();
    if CLAUSE_THEN_WORD_PATTERN.matches_clause_first_word(tail_clause) {
        tail_clause = tail_clause.from(1).trimmed();
    }
    if tail_clause.is_empty() {
        return Ok(None);
    }
    let tail_tokens = tail_clause.tokens();

    let Some((verb, verb_idx)) = find_verb(tail_tokens) else {
        return Ok(None);
    };
    if verb_idx != 0 {
        return Ok(None);
    }

    let rest_clause = tail_clause.from(1).trimmed();
    if !is_simple_chosen_object_reference(rest_clause.tokens()) {
        return Ok(None);
    }

    let effect = parse_effect_with_verb(verb, None, target_tokens)?;
    Ok(Some(effect))
}

pub(crate) fn parse_copy_spell_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    fn find_choose_new_targets_split_idx(tail: &[OwnedLexToken]) -> Option<usize> {
        for idx in 0..tail.len() {
            if !CLAUSE_AND_WORD_PATTERN.matches_token(&tail[idx]) {
                continue;
            }
            let after = normalized_copy_retarget_tail_clause(&tail[idx + 1..], false);
            if after.first_word() == Some("choose")
                && after.contains_any_word(&["target", "targets"])
                && after.contains_word("copy")
            {
                return Some(idx);
            }
        }
        None
    }

    fn normalized_copy_retarget_tail_clause(
        tokens: &[OwnedLexToken],
        keep_may: bool,
    ) -> LexedClause<'_> {
        let mut clause = LexedClause::new(tokens).trimmed();
        if let Some(rest) = clause.strip_prefix_clause(CLAUSE_YOU_PREFIX) {
            clause = rest.trimmed();
        } else if let Some(rest) = clause.strip_prefix_clause(CLAUSE_THAT_PLAYER_PREFIX) {
            clause = rest.trimmed();
        }
        if !keep_may && let Some(rest) = clause.strip_prefix_clause(CLAUSE_MAY_PREFIX) {
            clause = rest.trimmed();
        }
        clause
    }

    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if clause_words
        .windows(2)
        .any(|words| matches!(words, ["emblem", "with"]))
    {
        return Ok(None);
    }
    let Some(copy_idx) = find_token_index(tokens, |token| {
        CLAUSE_COPY_OR_COPIES_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let tail = &tokens[copy_idx + 1..];
    let split_idx = find_choose_new_targets_split_idx(tail);
    let exception_idx = word_slice_find_word_where(&clause_words, |word| {
        CLAUSE_EXCEPT_WORD_PATTERN.matches_word(word)
    });
    let clause_words_before_exception = exception_idx
        .map(|idx| &clause_words[..idx])
        .unwrap_or(&clause_words);
    let simple_copy_reference = copy_idx == 0
        && (clause_words
            .get(1)
            .copied()
            .is_some_and(|word| CLAUSE_COPY_REFERENCE_HEAD_WORD_PATTERN.matches_word(word))
            || CLAUSE_SIMPLE_COPY_REFERENCE_PATTERN.matches_words(&clause_words));
    if simple_copy_reference {
        let trailing_if = split_trailing_if_clause_lexed(tokens);
        let copy_clause_tokens = trailing_if
            .as_ref()
            .map_or(tokens, |spec| spec.leading_tokens);
        let Some(copy_clause_copy_idx) = find_token_index(copy_clause_tokens, |token| {
            CLAUSE_COPY_OR_COPIES_WORD_PATTERN.matches_token(token)
        }) else {
            return Ok(None);
        };
        let copy_clause_tail = &copy_clause_tokens[copy_clause_copy_idx + 1..];
        let copy_clause_split_idx = find_choose_new_targets_split_idx(copy_clause_tail);

        if let Some(then_idx) = find_token_index(copy_clause_tokens, |token| {
            CLAUSE_THEN_WORD_PATTERN.matches_token(token)
        }) {
            let tail_clause = LexedClause::new(&copy_clause_tokens[then_idx + 1..]).trimmed();
            if let Some(spec) =
                super::super::activation_and_restrictions::parse_may_cast_it_sentence(
                    tail_clause.tokens(),
                )
                && spec.as_copy
            {
                return Ok(Some(
                    super::super::activation_and_restrictions::build_may_cast_tagged_effect(&spec),
                ));
            }
        }
        let mut count = Value::Fixed(1);
        let copy_clause_exception_idx = find_token_index(copy_clause_tail, |token| {
            CLAUSE_EXCEPT_WORD_PATTERN.matches_token(token)
        });
        let copy_target_tail = if let Some(idx) = copy_clause_split_idx {
            &copy_clause_tail[..idx]
        } else if let Some(idx) = copy_clause_exception_idx {
            &copy_clause_tail[..idx]
        } else {
            copy_clause_tail
        };
        let (copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
        if let Some(count_value) = explicit_count {
            count = count_value;
        }
        if let Some(for_each_idx) = find_token_word_sequence(copy_target_tail, &["for", "each"]) {
            let copy_target_clause = LexedClause::new(copy_target_tail);
            let count_filter_clause = copy_target_clause
                .after_words(for_each_idx + 2)
                .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
                .trimmed();
            if count_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing count filter after 'for each' in copy clause (clause: '{}')",
                    clause_text
                )));
            }
            let count_filter = parse_object_filter(count_filter_clause.tokens(), false)?;
            count = Value::Count(count_filter);
        }
        let target_words = LexedClause::new(copy_target_tail).word_refs();
        let target = if CLAUSE_THIS_SPELL_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::Source(None)
        } else if CLAUSE_THAT_SPELL_OR_ABILITY_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::Tagged(TagKey::from("triggering"), None)
        } else if CLAUSE_THAT_SPELL_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::Tagged(TagKey::from("triggering"), None)
        } else if CLAUSE_THAT_ABILITY_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::Tagged(TagKey::from("triggering_source"), None)
        } else if CLAUSE_TAGGED_COPY_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::Tagged(TagKey::from(IT_TAG), None)
        } else {
            TargetAst::Source(None)
        };
        let base = EffectAst::subject_verb_copy_spell(
            target,
            count,
            PlayerAst::Implicit,
            copy_clause_split_idx.is_some(),
            parse_copy_spell_removed_supertypes(copy_clause_tail),
        );
        if let Some(trailing_if) = trailing_if {
            return Ok(Some(EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![base],
                if_false: Vec::new(),
            }));
        }
        return Ok(Some(base));
    }
    if !CLAUSE_SPELL_OR_ABILITY_MARKER_PATTERN.matches_words(clause_words_before_exception) {
        return Ok(None);
    }

    let subject = parse_subject(&tokens[..copy_idx]);
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let mut count = Value::Fixed(1);
    let exception_split_idx = find_token_index(tail, |token| {
        CLAUSE_EXCEPT_WORD_PATTERN.matches_token(token)
    });
    let mut copy_target_tail = if let Some(idx) = split_idx {
        &tail[..idx]
    } else if let Some(idx) = exception_split_idx {
        &tail[..idx]
    } else {
        tail
    };
    let (stripped_copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
    copy_target_tail = stripped_copy_target_tail;
    if let Some(count_value) = explicit_count {
        count = count_value;
    }
    if let Some(for_each_idx) = find_token_word_sequence(copy_target_tail, &["for", "each"]) {
        let copy_target_clause = LexedClause::new(copy_target_tail);
        let count_filter_clause = copy_target_clause
            .after_words(for_each_idx + 2)
            .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
            .trimmed();
        if count_filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing count filter after 'for each' in copy clause (clause: '{}')",
                clause_text
            )));
        }
        let count_filter = parse_object_filter(count_filter_clause.tokens(), false)?;
        count = Value::Count(count_filter);
        copy_target_tail = &copy_target_tail[..for_each_idx];
    }

    let copy_target_clause = LexedClause::new(copy_target_tail).trimmed();
    if copy_target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let target_words = copy_target_clause.word_refs();
    let target = if CLAUSE_THIS_SPELL_TARGET_PATTERN.matches_words(&target_words) {
        TargetAst::Source(None)
    } else if CLAUSE_THAT_SPELL_OR_ABILITY_TARGET_PATTERN.matches_words(&target_words) {
        TargetAst::Tagged(TagKey::from("triggering"), None)
    } else if CLAUSE_THAT_SPELL_TARGET_PATTERN.matches_words(&target_words) {
        TargetAst::Tagged(TagKey::from("triggering"), None)
    } else if CLAUSE_THAT_ABILITY_TARGET_PATTERN.matches_words(&target_words) {
        TargetAst::Tagged(TagKey::from("triggering_source"), None)
    } else if CLAUSE_TAGGED_COPY_TARGET_PATTERN.matches_words(&target_words) {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        parse_counter_target_phrase(copy_target_clause.tokens())?
    };

    let mut may_choose_new_targets = false;
    if let Some(idx) = split_idx {
        let raw_choose_clause = normalized_copy_retarget_tail_clause(&tail[idx + 1..], true);
        let choose_clause =
            if let Some(rest) = raw_choose_clause.strip_prefix_clause(CLAUSE_MAY_PREFIX) {
                may_choose_new_targets = true;
                rest.trimmed()
            } else {
                raw_choose_clause
            };
        let has_choose = CLAUSE_CHOOSE_WORD_PATTERN.matches_clause_first_word(choose_clause);
        let has_new = choose_clause.contains_word("new");
        let has_target = choose_clause.contains_any_word(&["target", "targets"]);
        let has_copy = choose_clause.contains_word("copy");
        if !has_choose || !has_target || !has_copy {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_text
            )));
        }
        if !has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_text
            )));
        }
    }

    Ok(Some(EffectAst::subject_verb_copy_spell(
        target,
        count,
        player,
        may_choose_new_targets,
        parse_copy_spell_removed_supertypes(tail),
    )))
}

fn parse_copy_spell_removed_supertypes(tokens: &[OwnedLexToken]) -> Vec<crate::types::Supertype> {
    let clause = LexedClause::new(tokens);
    if clause.contains_word("legendary") && clause.contains_any_word(&["except", "isnt"]) {
        vec![crate::types::Supertype::Legendary]
    } else {
        Vec::new()
    }
}

fn strip_copy_count_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Value>) {
    if token_slice_last_is(tokens, "twice") {
        return (
            &tokens[..tokens.len().saturating_sub(1)],
            Some(Value::Fixed(2)),
        );
    }
    (tokens, None)
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let clause = LexedClause::new(tokens);
    if clause.contains_word("ability") && clause.contains_any_word(&["activated", "triggered"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            clause.text()
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let mut clause_tokens = LexedClause::new(tokens).trim();
    if clause_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("counter"))
    {
        clause_tokens.drain(..1);
    }
    let clause = LexedClause::new(&clause_tokens);
    let is_opponents_control_tail = |idx: usize| {
        (clause_tokens
            .get(idx)
            .is_some_and(|token| token.as_word() == Some("your"))
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.as_word() == Some("opponents"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)))
            || (clause_tokens
                .get(idx)
                .is_some_and(|token| matches!(token.as_word(), Some("opponents" | "opponent")))
                && clause_tokens.get(idx + 1).is_some_and(|token| {
                    CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)
                }))
            || (clause_tokens
                .get(idx)
                .is_some_and(|token| token.as_word() == Some("an"))
                && clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.as_word() == Some("opponent"))
                && clause_tokens.get(idx + 2).is_some_and(|token| {
                    CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)
                }))
    };
    let is_controller_tail =
        |idx: usize| {
            clause_tokens
                .get(idx)
                .is_some_and(|token| CLAUSE_YOU_WORD_PATTERN.matches_token(token))
                && ((clause_tokens.get(idx + 1).is_some_and(|token| {
                    CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)
                })) || (clause_tokens.get(idx + 1).is_some_and(|token| {
                    CLAUSE_DONT_OR_DONT_APOSTROPHE_WORD_PATTERN.matches_token(token)
                }) && clause_tokens.get(idx + 2).is_some_and(|token| {
                    CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)
                })) || (clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| CLAUSE_DO_WORD_PATTERN.matches_token(token))
                    && clause_tokens
                        .get(idx + 2)
                        .is_some_and(|token| CLAUSE_NOT_WORD_PATTERN.matches_token(token))
                    && clause_tokens.get(idx + 3).is_some_and(|token| {
                        CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)
                    })))
                || is_opponents_control_tail(idx)
        };
    if clause.contains_no_words(&["ability", "abilities"]) {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if let Some((count, used)) = parse_choice_count_before_target_prefix(&clause_tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    }

    let explicit_target = clause_tokens
        .get(idx)
        .is_some_and(|token| CLAUSE_TARGET_WORD_PATTERN.matches_token(token));
    if explicit_target {
        idx += 1;
    } else if clause_tokens
        .get(idx)
        .is_some_and(|token| matches!(token.as_word(), Some("all" | "each")))
    {
        idx += 1;
    } else {
        return Ok(None);
    }

    #[derive(Clone, Copy)]
    enum CounterTargetTerm {
        Ability,
        Spell,
    }

    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if CLAUSE_FROM_WORD_PATTERN.matches_token(&clause_tokens[scan]) || is_controller_tail(scan)
        {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    fn parse_counter_term_at(
        tokens: &[OwnedLexToken],
        idx: usize,
    ) -> Option<(Vec<(ObjectFilter, CounterTargetTerm)>, usize)> {
        let make_triggered = || {
            let mut f = ObjectFilter::ability();
            f.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            f
        };

        if token_slice_starts_with(&tokens[idx..], &["activated", "or", "triggered", "ability"]) {
            return Some((
                vec![
                    (
                        ObjectFilter::activated_ability(),
                        CounterTargetTerm::Ability,
                    ),
                    (make_triggered(), CounterTargetTerm::Ability),
                ],
                4,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["triggered", "or", "activated", "ability"]) {
            return Some((
                vec![
                    (make_triggered(), CounterTargetTerm::Ability),
                    (
                        ObjectFilter::activated_ability(),
                        CounterTargetTerm::Ability,
                    ),
                ],
                4,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["activated", "ability"]) {
            return Some((
                vec![(
                    ObjectFilter::activated_ability(),
                    CounterTargetTerm::Ability,
                )],
                2,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["triggered", "ability"]) {
            return Some((vec![(make_triggered(), CounterTargetTerm::Ability)], 2));
        }
        if token_slice_starts_with(&tokens[idx..], &["instant", "spell"]) {
            return Some((
                vec![(
                    ObjectFilter::spell().with_type(crate::types::CardType::Instant),
                    CounterTargetTerm::Spell,
                )],
                2,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["sorcery", "spell"]) {
            return Some((
                vec![(
                    ObjectFilter::spell().with_type(crate::types::CardType::Sorcery),
                    CounterTargetTerm::Spell,
                )],
                2,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["legendary", "spell"]) {
            return Some((
                vec![(
                    ObjectFilter::spell().with_supertype(Supertype::Legendary),
                    CounterTargetTerm::Spell,
                )],
                2,
            ));
        }
        if token_slice_starts_with(&tokens[idx..], &["noncreature", "spell"]) {
            let mut f = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            f.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            return Some((vec![(f, CounterTargetTerm::Spell)], 2));
        }
        if token_slice_starts_with(&tokens[idx..], &["colorless", "spell"]) {
            return Some((
                vec![(ObjectFilter::spell().colorless(), CounterTargetTerm::Spell)],
                2,
            ));
        }
        if tokens
            .get(idx)
            .is_some_and(|token| matches!(token.as_word(), Some("ability" | "abilities")))
        {
            return Some((
                vec![(ObjectFilter::ability(), CounterTargetTerm::Ability)],
                1,
            ));
        }
        if tokens
            .get(idx)
            .is_some_and(|token| token.as_word() == Some("spell"))
        {
            return Some((vec![(ObjectFilter::spell(), CounterTargetTerm::Spell)], 1));
        }
        None
    }

    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();
    while idx < list_end {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if CLAUSE_AND_OR_OR_WORD_PATTERN.matches_word(word) {
            idx += 1;
            continue;
        }
        let Some((group, used)) = parse_counter_term_at(&clause_tokens, idx) else {
            return Ok(None);
        };
        term_filters.extend(group);
        idx += used;
    }

    if term_filters.is_empty() {
        return Ok(None);
    }

    let mut source_types: Vec<crate::types::CardType> = Vec::new();
    let mut controller_filter: Option<crate::target::PlayerFilter> = None;
    while idx < clause_tokens.len() {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if CLAUSE_AND_OR_OR_WORD_PATTERN.matches_word(word) {
            idx += 1;
            continue;
        }
        if CLAUSE_YOU_WORD_PATTERN.matches_word(word)
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token))
        {
            controller_filter = Some(crate::target::PlayerFilter::You);
            idx += 2;
            continue;
        }
        if CLAUSE_YOU_WORD_PATTERN.matches_word(word)
            && clause_tokens.get(idx + 1).is_some_and(|token| {
                CLAUSE_DONT_OR_DONT_APOSTROPHE_WORD_PATTERN.matches_token(token)
            })
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token))
        {
            controller_filter = Some(crate::target::PlayerFilter::NotYou);
            idx += 3;
            continue;
        }
        if CLAUSE_YOU_WORD_PATTERN.matches_word(word)
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| CLAUSE_DO_WORD_PATTERN.matches_token(token))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| CLAUSE_NOT_WORD_PATTERN.matches_token(token))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| CLAUSE_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token))
        {
            controller_filter = Some(crate::target::PlayerFilter::NotYou);
            idx += 4;
            continue;
        }
        if is_opponents_control_tail(idx) {
            controller_filter = Some(crate::target::PlayerFilter::Opponent);
            idx += if clause_tokens
                .get(idx)
                .is_some_and(|token| matches!(token.as_word(), Some("your" | "an")))
            {
                3
            } else {
                2
            };
            continue;
        }
        if CLAUSE_FROM_WORD_PATTERN.matches_word(word) {
            idx += 1;
            if clause_tokens
                .get(idx)
                .is_some_and(|token| CLAUSE_ARTICLE_WORD_PATTERN.matches_token(token))
            {
                idx += 1;
            }

            let mut parsed_type = false;
            while idx < clause_tokens.len() {
                let Some(type_word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word)
                else {
                    idx += 1;
                    continue;
                };
                if CLAUSE_SOURCE_OR_SOURCES_WORD_PATTERN.matches_word(type_word) {
                    idx += 1;
                    break;
                }
                if CLAUSE_AND_OR_OR_WORD_PATTERN.matches_word(type_word) {
                    idx += 1;
                    continue;
                }
                let parsed = parse_card_type(type_word)
                    .or_else(|| strip_suffix_char(type_word, 's').and_then(parse_card_type));
                let Some(card_type) = parsed else {
                    return Ok(None);
                };
                source_types.push(card_type);
                parsed_type = true;
                idx += 1;
            }
            if !parsed_type {
                return Ok(None);
            }
            continue;
        }

        return Ok(None);
    }

    for (filter, term) in &mut term_filters {
        if let Some(controller) = controller_filter.clone() {
            let mut updated = filter.clone();
            updated.controller = Some(controller);
            *filter = updated;
        }
        if !source_types.is_empty() && matches!(term, CounterTargetTerm::Ability) {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }

    let target_filter = if term_filters.len() == 1 {
        term_filters
            .pop()
            .map(|(filter, _)| filter)
            .expect("single term filter should be present")
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = term_filters.into_iter().map(|(filter, _)| filter).collect();
        any
    };

    let target = wrap_target_count(
        TargetAst::Object(
            target_filter,
            explicit_target
                .then(|| span_from_tokens(&clause_tokens))
                .flatten(),
            None,
        ),
        target_count,
    );
    Ok(Some(target))
}

pub(crate) fn parse_prevent_all_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    let Some(shape) = classify_prevent_all_damage_clause(&clause_words) else {
        return Ok(None);
    };

    if let PreventAllDamageClauseShape::DurationFirstSource { prefix_len } = shape {
        let source_clause = clause
            .after_words(prefix_len)
            .unwrap_or_else(|| clause.from(tokens.len()))
            .trimmed();
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage source filter (clause: '{}')",
                clause_text
            )));
        }
        let source_filter_clause = if CLAUSE_SOURCES_SUFFIX_PATTERN.matches(source_clause) {
            source_clause
                .strip_suffix_clause(CLAUSE_SOURCES_SUFFIX)
                .unwrap_or(source_clause)
                .trimmed()
        } else {
            source_clause
        };
        if source_filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage source phrase (clause: '{}')",
                clause_text
            )));
        }
        let source_filter_target = parse_target_phrase(source_filter_clause.tokens())?;
        let TargetAst::Object(source_filter, _, _) = source_filter_target else {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage source filter target (clause: '{}')",
                clause_text
            )));
        };
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_damage_from_source_filter(
                source_filter,
                Until::EndOfTurn,
            ),
        ));
    }

    if let PreventAllDamageClauseShape::TargetFirstSource {
        prefix_len,
        this_turn_idx,
        by_idx,
    } = shape
    {
        let target_clause = clause.between_words_trimmed(prefix_len, this_turn_idx);
        let source_clause = clause
            .after_words(by_idx + 1)
            .unwrap_or_else(|| clause.from(tokens.len()))
            .trimmed();
        if target_clause.is_empty() || source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage target or source filter (clause: '{}')",
                clause_text
            )));
        }

        let target = parse_target_phrase(target_clause.tokens())?;
        if CLAUSE_SOURCE_OF_YOUR_CHOICE_MARKER_PATTERN.matches(source_clause) {
            let target_words = target_clause.word_refs();
            if !matches!(target_words.as_slice(), ["you"]) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported prevent-all damage source choice target (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_damage_to_target_with_source_choice(
                    target,
                    Until::EndOfTurn,
                    true,
                ),
            ));
        }
        let source_filter_target = parse_target_phrase(source_clause.tokens())?;
        let TargetAst::Object(source_filter, _, _) = source_filter_target else {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage source filter target (clause: '{}')",
                clause_text
            )));
        };

        return Ok(Some(
            EffectAst::subject_verb_prevent_all_damage_to_target_from_source_filter(
                target,
                source_filter,
                Until::EndOfTurn,
            ),
        ));
    }

    let target_clause = match shape {
        PreventAllDamageClauseShape::DurationFirstTarget { prefix_len } => clause
            .after_words(prefix_len)
            .unwrap_or_else(|| clause.from(tokens.len()))
            .trimmed(),
        PreventAllDamageClauseShape::TargetFirst { prefix_len } => {
            if clause_words.len() <= prefix_len + 1 {
                return Err(CardTextError::ParseError(format!(
                    "missing prevent-all damage target (clause: '{}')",
                    clause_text
                )));
            }
            clause.between_words_trimmed(prefix_len, clause_words.len() - 2)
        }
        PreventAllDamageClauseShape::DurationFirstSource { .. } => {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage target (clause: '{}')",
                clause_text
            )));
        }
        PreventAllDamageClauseShape::TargetFirstSource { .. } => unreachable!(
            "target-plus-source prevent-all damage clauses are handled before target-only lowering"
        ),
    };
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_text
        )));
    }

    let target = parse_target_phrase(target_clause.tokens())?;

    Ok(Some(EffectAst::subject_verb_prevent_all_damage_to_target(
        target,
        Until::EndOfTurn,
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreventAllDamageClauseShape {
    DurationFirstSource {
        prefix_len: usize,
    },
    DurationFirstTarget {
        prefix_len: usize,
    },
    TargetFirst {
        prefix_len: usize,
    },
    TargetFirstSource {
        prefix_len: usize,
        this_turn_idx: usize,
        by_idx: usize,
    },
}

fn classify_prevent_all_damage_clause(words: &[&str]) -> Option<PreventAllDamageClauseShape> {
    if let Some(prefix_len) =
        CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_BY_PATTERN.matched_prefix_len(words)
    {
        return Some(PreventAllDamageClauseShape::DurationFirstSource { prefix_len });
    }
    if let Some(prefix_len) =
        CLAUSE_PREVENT_ALL_DAMAGE_THIS_TURN_TO_PATTERN.matched_prefix_len(words)
    {
        return Some(PreventAllDamageClauseShape::DurationFirstTarget { prefix_len });
    }
    if let Some(prefix_len) = CLAUSE_PREVENT_ALL_DAMAGE_TO_PATTERN.matched_prefix_len(words)
        && CLAUSE_PREVENT_ALL_DAMAGE_TO_PATTERN.matches_words(words)
    {
        return Some(PreventAllDamageClauseShape::TargetFirst { prefix_len });
    }
    if let Some(prefix_len) = CLAUSE_PREVENT_ALL_DAMAGE_TO_PATTERN.matched_prefix_len(words)
        && let Some(this_turn_rel) = CLAUSE_THIS_TURN_BY_PATTERN
            .find_exact_window(words.get(prefix_len..).unwrap_or_default(), 3)
    {
        let this_turn_idx = prefix_len + this_turn_rel;
        return Some(PreventAllDamageClauseShape::TargetFirstSource {
            prefix_len,
            this_turn_idx,
            by_idx: this_turn_idx + 2,
        });
    }
    None
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let Some(can_idx) = clause.find_word("can") else {
        return Ok(None);
    };
    let tail_clause = clause.after_words(can_idx).unwrap_or(clause).trimmed();
    let tail_words = tail_clause.word_refs();
    let has_full_core = CLAUSE_CAN_ATTACK_PREFIX_PATTERN.matches_words(&tail_words)
        && CLAUSE_AS_THOUGH_MARKER_PATTERN.matches_words(&tail_words)
        && CLAUSE_TURN_MARKER_PATTERN.matches_words(&tail_words)
        && CLAUSE_HAVE_MARKER_PATTERN.matches_words(&tail_words)
        && CLAUSE_DEFENDER_TAIL_PATTERN.matches_words(&tail_words);
    let has_split_core = CLAUSE_CAN_ATTACK_PREFIX_PATTERN.matches_words(&tail_words)
        && CLAUSE_AS_THOUGH_MARKER_PATTERN.matches_words(&tail_words)
        && CLAUSE_TURN_MARKER_PATTERN.matches_words(&tail_words)
        && CLAUSE_DIDNT_TAIL_PATTERN.matches_words(&tail_words);
    if !has_full_core && !has_split_core {
        return Ok(None);
    }

    let subject_clause = clause
        .before_word(can_idx)
        .unwrap_or(clause.before(0))
        .trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanAttackAsThoughNoDefender],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_prevent_next_time_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !CLAUSE_THE_NEXT_TIME_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(would_idx) = clause.find_word("would") else {
        return Ok(None);
    };
    let clause_words = clause.word_refs();
    let damage_target_start =
        if CLAUSE_DEAL_DAMAGE_TO_PREFIX_PATTERN.matches_words(&clause_words[would_idx + 1..]) {
            would_idx + 4
        } else if word_slice_starts_with(&clause_words[would_idx + 1..], &["deal", "damage"]) {
            would_idx + 3
        } else {
            return Ok(None);
        };

    let this_turn_rel = CLAUSE_THIS_TURN_PATTERN
        .find_exact_window(&clause_words[damage_target_start..], 2)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next-time damage duration (clause: '{}')",
                clause_text
            ))
        })?;
    let this_turn_idx = damage_target_start + this_turn_rel;

    let tail_clause = clause
        .after_words(this_turn_idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()))
        .trimmed();
    let reflect_damage_to_source_controller =
        if CLAUSE_PREVENT_THAT_DAMAGE_PATTERN.matches(tail_clause) {
            false
        } else if CLAUSE_PREVENT_THAT_DAMAGE_IF_PREVENTED_PREFIX_PATTERN.matches(tail_clause) {
            true
        } else {
            return Ok(None);
        };

    let source_clause = clause.between_words_trimmed(3, would_idx);
    let source_words = source_clause.word_refs();
    if source_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next-time damage source (clause: '{}')",
            clause_text
        )));
    }

    let source = if CLAUSE_SOURCE_OF_YOUR_CHOICE_MARKER_PATTERN.matches(source_clause) {
        PreventNextTimeDamageSourceAst::Choice
    } else if matches!(source_words.as_slice(), ["it"] | ["that", _]) {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        if let ["that", kind] = source_words.as_slice()
            && let Some(card_type) = parse_card_type(kind)
        {
            filter.card_types.push(card_type);
        }
        PreventNextTimeDamageSourceAst::Target(TargetAst::Object(
            filter,
            None,
            span_from_tokens(source_clause.tokens()),
        ))
    } else {
        let mut words = strip_leading_article_word_refs(&source_words).to_vec();
        if CLAUSE_SOURCE_WORD_PATTERN.matches_last_word(&words) {
            words.pop();
        }
        if words.is_empty() {
            let effect = if reflect_damage_to_source_controller {
                EffectAst::subject_verb_prevent_next_time_damage_with_reflection(
                    PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                    PreventNextTimeDamageTargetAst::AnyTarget,
                    true,
                )
            } else {
                EffectAst::subject_verb_prevent_next_time_damage(
                    PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                    PreventNextTimeDamageTargetAst::AnyTarget,
                )
            };
            return Ok(Some(vec![effect]));
        }

        let mut filter = ObjectFilter::default();
        let mut colors: Option<crate::color::ColorSet> = None;
        for w in words {
            if CLAUSE_AND_OR_WORD_PATTERN.matches_word(w) {
                continue;
            }
            if let Some(color) = parse_color(w) {
                colors = Some(
                    colors
                        .unwrap_or_else(crate::color::ColorSet::new)
                        .union(color),
                );
                continue;
            }
            if let Some(card_type) = parse_card_type(w) {
                push_unique(&mut filter.card_types, card_type);
                continue;
            }
            if CLAUSE_SHADOW_WORD_PATTERN.matches_word(w) {
                filter = filter.with_static_ability(StaticAbilityId::Shadow);
                continue;
            }
        }
        if let Some(colors) = colors {
            filter.colors = Some(colors);
        }

        PreventNextTimeDamageSourceAst::Filter(filter)
    };

    let target_clause = clause.between_words_trimmed(damage_target_start, this_turn_idx);
    let target = if target_clause.is_empty() {
        PreventNextTimeDamageTargetAst::AnyTarget
    } else if CLAUSE_YOU_TARGET_PATTERN.matches(target_clause) {
        PreventNextTimeDamageTargetAst::You
    } else if CLAUSE_ANY_TARGET_PATTERN.matches(target_clause) {
        PreventNextTimeDamageTargetAst::AnyTarget
    } else if !target_clause.is_empty() {
        PreventNextTimeDamageTargetAst::Target(parse_target_phrase(target_clause.tokens())?)
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next-time damage target scope (clause: '{}')",
            clause_text
        )));
    };

    let effect = if reflect_damage_to_source_controller {
        EffectAst::subject_verb_prevent_next_time_damage_with_reflection(source, target, true)
    } else {
        EffectAst::subject_verb_prevent_next_time_damage(source, target)
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_redirect_next_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if CLAUSE_REDIRECT_DAMAGE_PREFIX_PATTERN.matches_words(&clause_words) {
        let target_start = 9usize;
        let is_dealt_rel = CLAUSE_IS_DEALT_TO_PREFIX_PATTERN
            .find_exact_window(&clause_words[target_start..], 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-all-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let is_dealt_idx = target_start + is_dealt_rel;
        let protected_words = &clause_words[target_start..is_dealt_idx];
        let object_filter = match protected_words {
            ["you", "and", "permanents", "you", "control"]
            | ["you", "and", "permanent", "you", "control"] => {
                ObjectFilter::permanent().you_control()
            }
            ["you", "and", "other", "permanents", "you", "control"]
            | ["you", "and", "other", "permanent", "you", "control"] => {
                ObjectFilter::permanent().you_control().other()
            }
            _ => return Ok(None),
        };

        let redirect_words = &clause_words[is_dealt_idx + 3..];
        if !CLAUSE_INSTEAD_SUFFIX_PATTERN.matches_words(redirect_words) || redirect_words.len() < 2
        {
            return Ok(None);
        }
        let target_word_start = is_dealt_idx + 3;
        let target_word_end = clause_words.len() - 1;
        let target_token_start =
            token_index_for_word_index(tokens, target_word_start).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-all-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let target_token_end =
            token_index_for_word_index(tokens, target_word_end).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-all-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let target =
            parse_target_phrase(&trim_commas(&tokens[target_token_start..target_token_end]))?;

        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_target(
                PlayerFilter::You,
                object_filter,
                target,
            ),
        ]));
    }

    if clause_words.starts_with(&[
        "all", "damage", "that", "would", "be", "dealt", "this", "turn", "by",
    ]) {
        let is_dealt_rel = clause
            .from_word(9)
            .unwrap_or_else(|| clause.from(clause.len()))
            .find_phrase_start(&["is", "dealt", "to"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-source-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let is_dealt_idx = 9 + is_dealt_rel;
        let source_clause = clause.between_words_trimmed(9, is_dealt_idx);
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-source-damage source (clause: '{}')",
                clause_text
            )));
        }

        let redirect_clause = clause
            .after_words(is_dealt_idx + 3)
            .unwrap_or_else(|| clause.from(clause.len()))
            .trimmed();
        if !redirect_clause.matches_any_words(&[
            &["that", "spell's", "controller", "instead"],
            &["that", "spells", "controller", "instead"],
            &["that", "source's", "controller", "instead"],
            &["that", "sources", "controller", "instead"],
        ]) {
            return Ok(None);
        }

        let source = parse_target_phrase(source_clause.tokens())?;
        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_all_damage_this_turn_by_source_to_source_controller(
                source,
            ),
        ]));
    }

    if CLAUSE_ALL_DAMAGE_WOULD_BE_DEALT_TO_PREFIX_PATTERN.matches(clause) {
        let idx = 7usize;
        let this_turn_rel = LexedClause::new(
            clause
                .from_word(idx)
                .unwrap_or_else(|| clause.from(clause.len()))
                .tokens(),
        )
        .find_phrase_start(&["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-all-damage duration (clause: '{}')",
                clause_text
            ))
        })?;
        let this_turn_idx = idx + this_turn_rel;
        let target_clause = clause.between_words_trimmed(idx, this_turn_idx);
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-all-damage target (clause: '{}')",
                clause_text
            )));
        }

        let by_idx = this_turn_idx + 2;
        if !clause_words
            .get(by_idx)
            .is_some_and(|word| CLAUSE_BY_WORD_PATTERN.matches_word(word))
        {
            return Ok(None);
        }
        let is_dealt_rel = clause
            .from_word(by_idx + 1)
            .unwrap_or_else(|| clause.from(clause.len()))
            .find_phrase_start(&["is", "dealt", "to"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-all-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let is_dealt_idx = by_idx + 1 + is_dealt_rel;

        let source_clause = clause.between_words_trimmed(by_idx + 1, is_dealt_idx);
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-all-damage source (clause: '{}')",
                clause_text
            )));
        }

        let source = if CLAUSE_SOURCE_OF_YOUR_CHOICE_MARKER_PATTERN.matches(source_clause) {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-all-damage source scope (clause: '{}')",
                clause_text
            )));
        };

        let redirect_clause = clause
            .after_words(is_dealt_idx + 3)
            .unwrap_or_else(|| clause.from(clause.len()))
            .trimmed();
        let destination = if redirect_clause.matches_any_words(&[
            &["this", "creature", "instead"],
            &["this", "permanent", "instead"],
            &["this", "instead"],
            &["it", "instead"],
        ]) {
            RedirectNextTimeDamageDestinationAst::SourceObject
        } else if redirect_clause.matches_any_words(&[&["you", "instead"]]) {
            RedirectNextTimeDamageDestinationAst::Controller
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-all-damage protected destination (clause: '{}')",
                clause_text
            )));
        };

        let target = parse_target_phrase(target_clause.tokens())?;

        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_source(
                source,
                target,
                destination,
            ),
        ]));
    }

    if CLAUSE_THE_NEXT_TIME_PREFIX_PATTERN.matches(clause) {
        let Some(would_idx) = clause.find_word("would") else {
            return Ok(None);
        };
        if clause_words.get(would_idx + 1..would_idx + 4)
            != Some(["deal", "damage", "to"].as_slice())
        {
            return Ok(None);
        }

        let this_turn_rel = clause
            .from_word(would_idx + 4)
            .unwrap_or_else(|| clause.from(clause.len()))
            .find_phrase_start(&["this", "turn"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-next-time damage duration (clause: '{}')",
                    clause_text
                ))
            })?;
        let this_turn_idx = (would_idx + 4) + this_turn_rel;

        let source_clause = clause.between_words_trimmed(3, would_idx);
        let source_words = source_clause.word_refs();
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage source (clause: '{}')",
                clause_text
            )));
        }

        let source = if CLAUSE_SOURCE_OF_YOUR_CHOICE_MARKER_PATTERN.matches(source_clause) {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            let mut words = strip_leading_article_word_refs(&source_words).to_vec();
            if CLAUSE_SOURCE_WORD_PATTERN.matches_last_word(&words) {
                words.pop();
            }
            let mut filter = ObjectFilter::default();
            let mut colors: Option<crate::color::ColorSet> = None;
            for word in words {
                if CLAUSE_AND_OR_OR_WORD_PATTERN.matches_word(word) {
                    continue;
                }
                if let Some(color) = parse_color(word) {
                    colors = Some(
                        colors
                            .unwrap_or_else(crate::color::ColorSet::new)
                            .union(color),
                    );
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique(&mut filter.card_types, card_type);
                    continue;
                }
                if CLAUSE_SHADOW_WORD_PATTERN.matches_word(word) {
                    filter = filter.with_static_ability(StaticAbilityId::Shadow);
                    continue;
                }
            }
            if let Some(colors) = colors {
                filter.colors = Some(colors);
            }
            PreventNextTimeDamageSourceAst::Filter(filter)
        };

        let target_clause = clause.between_words_trimmed(would_idx + 4, this_turn_idx);
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage target (clause: '{}')",
                clause_text
            )));
        }
        let target = parse_target_phrase(target_clause.tokens())?;

        let tail_clause = clause
            .after_words(this_turn_idx + 2)
            .unwrap_or_else(|| clause.from(clause.len()))
            .trimmed();
        let destination_clause = if tail_clause.word_len() >= 7
            && CLAUSE_THAT_DAMAGE_IS_DEALT_TO_PREFIX_PATTERN.matches(tail_clause)
            && CLAUSE_INSTEAD_SUFFIX_PATTERN.matches(tail_clause)
        {
            tail_clause.between_words_trimmed(5, tail_clause.word_len() - 1)
        } else if tail_clause.word_len() >= 8
            && CLAUSE_THAT_SOURCE_DEALS_THAT_DAMAGE_TO_PREFIX_PATTERN.matches(tail_clause)
            && CLAUSE_INSTEAD_SUFFIX_PATTERN.matches(tail_clause)
        {
            tail_clause.between_words_trimmed(6, tail_clause.word_len() - 1)
        } else {
            return Ok(None);
        };
        let (destination, destination_target) = if destination_clause.matches_any_words(&[
            &["this"],
            &["it"],
            &["this", "creature"],
            &["this", "permanent"],
        ]) {
            (RedirectNextTimeDamageDestinationAst::SourceObject, None)
        } else if destination_clause.matches_any_words(&[&["you"]]) {
            (RedirectNextTimeDamageDestinationAst::Controller, None)
        } else if destination_clause
            .word_refs()
            .first()
            .is_some_and(|word| CLAUSE_TARGET_WORD_PATTERN.matches_word(word))
        {
            if destination_clause.contains_word("choice") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported redirected-next-time damage destination (clause: '{}')",
                    clause_text
                )));
            }
            (
                RedirectNextTimeDamageDestinationAst::TargetObject,
                Some(parse_target_phrase(destination_clause.tokens())?),
            )
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-next-time damage destination (clause: '{}')",
                clause_text
            )));
        };

        let effect = if let Some(destination_target) = destination_target {
            EffectAst::subject_verb_redirect_next_time_damage_to_target(
                source,
                target,
                destination_target,
            )
        } else {
            EffectAst::subject_verb_redirect_next_time_damage_to_source(source, target, destination)
        };

        return Ok(Some(vec![effect]));
    }

    if !CLAUSE_THE_NEXT_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(amount_token_idx) = clause.token_index_for_word_index(2) else {
        return Ok(None);
    };
    let amount_token = tokens[amount_token_idx].clone();
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Ok(None);
    };
    if amount_used != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage amount (clause: '{}')",
            clause_text
        )));
    }

    let mut idx = 3usize;
    if !clause
        .words()
        .slice_eq(idx, DAMAGE_THAT_WOULD_BE_DEALT_TO_WORDS)
    {
        return Ok(None);
    }
    idx += 6;

    let this_turn_rel = clause
        .from_word(idx)
        .unwrap_or_else(|| clause.from(clause.len()))
        .find_phrase_start(&["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-next-damage duration (clause: '{}')",
                clause_text
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let protected_clause = clause.between_words_trimmed(idx, this_turn_idx);
    let protects_source = protected_clause.matches_any_words(&[
        &["this"],
        &["it"],
        &["this", "creature"],
        &["this", "permanent"],
    ]);
    let protected_target = if protects_source {
        None
    } else {
        Some(parse_target_phrase(protected_clause.tokens())?)
    };

    let tail_clause = clause
        .after_words(this_turn_idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()))
        .trimmed();
    if tail_clause.word_len() < 5
        || !CLAUSE_IS_DEALT_TO_PREFIX_PATTERN.matches(tail_clause)
        || !CLAUSE_INSTEAD_SUFFIX_PATTERN.matches(tail_clause)
    {
        return Ok(None);
    }

    let target_clause = tail_clause.between_words_trimmed(3, tail_clause.word_len() - 1);
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing redirected-next-damage target (clause: '{}')",
            clause_text
        )));
    }
    let effect = if target_clause.matches_any_words(&[&["you"]]) {
        let protected_target = protected_target.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing redirected-next-damage protected target (clause: '{}')",
                clause_text
            ))
        })?;
        EffectAst::subject_verb_redirect_next_damage_to_controller(amount, protected_target)
    } else {
        let target = parse_target_phrase(target_clause.tokens())?;
        let mut effect =
            EffectAst::subject_verb_redirect_next_damage_from_source_to_target(amount, target);
        if let EffectAst::SubjectVerb(subject_verb) = &mut effect {
            if let SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target: effect_protected_target,
                ..
            } = &mut subject_verb.action
            {
                *effect_protected_target = protected_target;
            }
        }
        effect
    };

    Ok(Some(vec![effect]))
}

pub(crate) fn parse_can_block_additional_creature_this_turn_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let Some(can_idx) = clause.find_word("can") else {
        return Ok(None);
    };
    let tail_clause = clause.after_words(can_idx).unwrap_or(clause).trimmed();
    let tail_words = tail_clause.word_refs();
    if !CLAUSE_CAN_BLOCK_PREFIX_PATTERN.matches_words(&tail_words)
        || !word_slice_ends_with(&tail_words, &["this", "turn"])
    {
        return Ok(None);
    }

    let Some(additional_offset) = tail_words
        .iter()
        .position(|word| CLAUSE_ADDITIONAL_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if !tail_words
        .get(additional_offset + 1)
        .is_some_and(|word| CLAUSE_CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut additional = 1usize;
    if additional_offset > 0 {
        let number_word_idx = can_idx + additional_offset - 1;
        if !CLAUSE_ARTICLE_WORD_PATTERN.matches_word(clause_words[number_word_idx])
            && let Some(number_token_idx) = clause.token_index_for_word_index(number_word_idx)
            && let Some((parsed, used)) = parse_number(&tokens[number_token_idx..])
            && used > 0
        {
            additional = parsed as usize;
        }
    }

    let subject_clause = clause
        .before_word(can_idx)
        .unwrap_or(clause.before(0))
        .trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional }],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_win_the_game_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    if clause.word_len() < 4 || !CLAUSE_YOU_WIN_GAME_PATTERN.matches_words(&clause_words[..4]) {
        return Ok(None);
    }

    if clause.word_len() == 4 {
        return Ok(Some(EffectAst::subject_verb_win_game(PlayerAst::You)));
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
        let leading_clause = LexedClause::new(trailing_if.leading_tokens);
        let leading_words = leading_clause.word_refs();
        if leading_words.len() == 4 && CLAUSE_YOU_WIN_GAME_PATTERN.matches_words(&leading_words) {
            return Ok(Some(EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
                if_false: Vec::new(),
            }));
        }
    }

    let Some(if_tail_clause) = clause.after_words(4) else {
        return Ok(None);
    };
    if !if_tail_clause
        .first_word()
        .is_some_and(|word| CLAUSE_IF_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let if_tail_clause = if_tail_clause
        .after_words(1)
        .unwrap_or(if_tail_clause)
        .trimmed();
    let if_tail = if_tail_clause.word_refs();
    if if_tail.len() < 6
        || !CLAUSE_YOU_WORD_PATTERN.matches_word(if_tail[0])
        || !CLAUSE_OWN_WORD_PATTERN.matches_word(if_tail[1])
        || !CLAUSE_ARTICLE_WORD_PATTERN.matches_word(if_tail[2])
        || !CLAUSE_CARD_WORD_PATTERN.matches_word(if_tail[3])
        || !CLAUSE_NAMED_WORD_PATTERN.matches_word(if_tail[4])
    {
        return Ok(None);
    }

    let after_named = &if_tail[5..];
    let Some(in_idx) = word_slice_find_word_where(after_named, |word| {
        CLAUSE_IN_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    if in_idx == 0 {
        return Ok(None);
    }

    let name_words = &after_named[..in_idx];
    let remainder_clause = if_tail_clause
        .after_words(5 + in_idx)
        .unwrap_or_else(|| if_tail_clause.from(if_tail_clause.len()))
        .trimmed();

    let has_exile = remainder_clause.contains_word("exile");
    let has_hand = remainder_clause.contains_word("hand");
    let has_graveyard = remainder_clause.contains_word("graveyard");
    let has_battlefield = remainder_clause.contains_word("battlefield");
    if !(has_exile && has_hand && has_graveyard && has_battlefield) {
        return Ok(None);
    }

    let name = name_words
        .iter()
        .map(|word| title_case_token_word(word))
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(EffectAst::Conditional {
        predicate: crate::cards::builders::PredicateAst::PlayerOwnsCardNamedInZones {
            player: PlayerAst::You,
            name,
            zones: vec![Zone::Exile, Zone::Hand, Zone::Graveyard, Zone::Battlefield],
        },
        if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
        if_false: Vec::new(),
    }))
}

fn parse_choose_target_prelude_targets(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<TargetAst>>, CardTextError> {
    let Some((first, second)) = grammar::split_lexed_once_on_separator(target_tokens, || {
        use winnow::Parser as _;
        grammar::kw("and").void()
    }) else {
        return Ok(None);
    };
    let first = trim_commas(first);
    let second = trim_commas(second);
    if first.is_empty() || second.is_empty() || !starts_with_target_indicator(&second) {
        return Ok(None);
    }

    Ok(Some(vec![
        parse_target_phrase(&first)?,
        parse_target_phrase(&second)?,
    ]))
}

fn parse_kicked_additional_targets_prelude(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn canonical_target_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
        LexedClause::new(tokens)
            .word_refs()
            .into_iter()
            .filter(|word| !matches!(*word, "another" | "other" | "target" | "a" | "an" | "the"))
            .collect()
    }

    let Some(then_choose_idx) = find_token_word_sequence(target_tokens, &["then", "choose"]) else {
        return Ok(None);
    };

    let first_target_tokens = trim_commas(&target_tokens[..then_choose_idx]);
    let after_choose = trim_commas(&target_tokens[then_choose_idx + 2..]);
    if first_target_tokens.is_empty() || after_choose.is_empty() {
        return Ok(None);
    }

    let Some(for_each_idx) = find_token_word_sequence(
        &after_choose,
        &["for", "each", "time", "this", "spell", "was", "kicked"],
    ) else {
        return Ok(None);
    };

    let additional_target_tokens = trim_commas(&after_choose[..for_each_idx]);
    let suffix_words = LexedClause::new(&after_choose[for_each_idx..]).word_refs();
    if additional_target_tokens.is_empty()
        || !word_slice_eq(
            &suffix_words,
            &["for", "each", "time", "this", "spell", "was", "kicked"],
        )
    {
        return Ok(None);
    }

    if canonical_target_words(&first_target_tokens)
        != canonical_target_words(&additional_target_tokens)
    {
        return Ok(None);
    }

    let first_target = parse_target_phrase(&first_target_tokens)?;
    let count = Value::Add(Box::new(Value::Fixed(1)), Box::new(Value::KickCount));
    Ok(Some(vec![EffectAst::subject_verb_target_only(
        TargetAst::WithCountValue(Box::new(first_target), ChoiceCount::dynamic_x(), count),
    )]))
}

pub(crate) fn parse_choose_target_prelude_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !CLAUSE_CHOOSE_WORD_PATTERN.matches_clause_first_word(clause) {
        return Ok(None);
    }

    let target_clause = clause.from(1).trimmed();
    let target_tokens = target_clause.tokens();
    if target_clause.is_empty() || !starts_with_target_indicator(target_tokens) {
        return Ok(None);
    }
    if find_verb(target_tokens).is_some() {
        return Ok(None);
    }

    if let Some(effects) = parse_kicked_additional_targets_prelude(target_tokens)? {
        return Ok(Some(effects));
    }

    if let Some(targets) = parse_choose_target_prelude_targets(target_tokens)? {
        return Ok(Some(
            targets
                .into_iter()
                .map(EffectAst::subject_verb_target_only)
                .collect(),
        ));
    }

    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(vec![EffectAst::subject_verb_target_only(target)]))
}

pub(crate) fn parse_keyword_mechanic_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut start = 0usize;
    if token_slice_at_is(tokens, start, "then") {
        start += 1;
    }
    if token_slice_at_is(tokens, start, "you") {
        start += 1;
    }
    if start >= tokens.len() {
        return Ok(None);
    }

    let clause = LexedClause::new(&tokens[start..]);
    let clause_tokens = clause.tokens();
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if clause.is_empty() {
        return Ok(None);
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_AMASS_WORD_PATTERN.matches_word(word))
    {
        let mut amount_start = 1usize;
        let mut subtype = None;

        if let Some(candidate) = clause_words.get(amount_start).copied()
            && let Some(parsed_subtype) = parse_subtype_word(candidate)
                .or_else(|| strip_suffix_char(candidate, 's').and_then(parse_subtype_word))
            && parsed_subtype.is_creature_type()
        {
            subtype = Some(parsed_subtype);
            amount_start += 1;
        }

        let (mut amount, used) = parse_value(&clause_tokens[amount_start..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for amass clause (clause: '{}')",
                clause_text
            ))
        })?;
        let trailing_tokens = LexedClause::new(&clause_tokens[amount_start + used..]).trim();
        if !trailing_tokens.is_empty() {
            let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing amass clause (clause: '{}')",
                    clause_text
                )));
            };
            amount = super::super::util::replace_unbound_x_with_value(
                amount,
                &where_value,
                &crate::runtime_backend::token_word_refs(&trailing_tokens).join(" "),
            )?;
        }

        return Ok(Some(EffectAst::subject_verb_amass(subtype, amount)));
    }

    if CLAUSE_FORAGE_WORD_PATTERN.matches_words(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::Forage,
            1,
        )));
    }

    if CLAUSE_HARNESS_WORD_PATTERN.matches_first_word(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::Harness,
            1,
        )));
    }

    if clause
        .first_word()
        .is_some_and(|word| CLAUSE_ROLL_WORD_PATTERN.matches_word(word))
        && CLAUSE_DICE_MARKER_PATTERN.matches_words(&clause_words)
    {
        if CLAUSE_DICE_WORD_PATTERN.matches_last_word(&clause_words)
            && clause_words.len() >= 5
            && CLAUSE_SIX_SIDED_PATTERN
                .matches_words(&clause_words[clause_words.len() - 3..clause_words.len() - 1])
        {
            let value_clause = clause.between_words_trimmed(1, clause_words.len() - 3);
            let value_tokens = value_clause.tokens();
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing roll-dice count (clause: '{}')",
                    clause_text
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported roll-dice count tail (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![EffectAst::subject_verb_roll_die(PlayerAst::Implicit, 6)],
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported roll-dice clause (clause: '{}')",
            clause_text
        )));
    }
    if let Some((prefix, tail_clause)) = clause.strip_any_prefix_clause(ODD_EVEN_RESULT_PREFIXES) {
        let predicate = if prefix == ODD_EVEN_RESULT_PREFIXES[0] {
            crate::effect::Comparison::OneOf(ODD_RESULT_VALUES_D6)
        } else {
            crate::effect::Comparison::OneOf(EVEN_RESULT_VALUES_D6)
        };
        let mut tail_clause = tail_clause.trimmed();
        while CLAUSE_THEN_OR_YOU_WORD_PATTERN.matches_clause_first_word(tail_clause) {
            tail_clause = tail_clause.from(1).trimmed();
        }
        let tail_tokens = tail_clause.tokens();
        let Some((verb, verb_idx)) = find_verb(tail_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "missing action after odd/even-result clause (clause: '{}')",
                clause_text
            )));
        };
        if verb_idx != 0 {
            return Err(CardTextError::ParseError(format!(
                "unsupported odd/even-result action prefix (clause: '{}')",
                clause_text
            )));
        }
        let effect = parse_effect_with_verb(verb, None, &tail_tokens[1..])?;
        return Ok(Some(EffectAst::IfResult {
            predicate: IfResultPredicate::Value(predicate),
            effects: vec![effect],
        }));
    }

    if CLAUSE_UNSUPPORTED_KEYWORD_EFFECT_WORD_PATTERN.matches_first_word(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some((_, mut target_clause)) =
        clause.strip_any_suffix_clause(&[&["phase", "out"], &["phases", "out"]])
        && !target_clause.is_empty()
    {
        target_clause = target_clause.trimmed();
        if target_clause
            .first_word()
            .is_some_and(|word| CLAUSE_SIMULTANEOUSLY_WORD_PATTERN.matches_word(word))
        {
            target_clause = target_clause.from(1).trimmed();
        }
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in phase-out clause (clause: '{}')",
                clause_text
            )));
        }
        if target_clause
            .first_word()
            .is_some_and(|word| CLAUSE_ALL_WORD_PATTERN.matches_word(word))
        {
            let filter_clause = target_clause.from(1).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-out clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_out_all(filter)));
        }
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(EffectAst::subject_verb_phase_out(target)));
    }

    if let Some((_, mut target_clause)) =
        clause.strip_any_suffix_clause(&[&["phase", "in"], &["phases", "in"]])
        && clause_tokens.len() >= 2
    {
        target_clause = target_clause.trimmed();
        if target_clause
            .first_word()
            .is_some_and(|word| CLAUSE_SIMULTANEOUSLY_WORD_PATTERN.matches_word(word))
        {
            target_clause = target_clause.from(1).trimmed();
        }
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in phase-in clause (clause: '{}')",
                clause_text
            )));
        }
        if target_clause
            .first_word()
            .is_some_and(|word| CLAUSE_ALL_WORD_PATTERN.matches_word(word))
            && target_clause
                .token(1)
                .is_some_and(|token| CLAUSE_PHASED_WORD_PATTERN.matches_token(token))
        {
            let filter_clause = target_clause.from(2).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-in clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_in_all(filter)));
        }
        if target_clause
            .first_word()
            .is_some_and(|word| CLAUSE_ALL_WORD_PATTERN.matches_word(word))
        {
            let filter_clause = target_clause.from(1).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-in clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_in_all(filter)));
        }
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(EffectAst::subject_verb_phase_in(target)));
    }

    if clause.starts_with_any(OPEN_ATTRACTION_PREFIXES) {
        return Ok(Some(EffectAst::subject_verb_open_attraction(
            PlayerAst::Implicit,
        )));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_BEHOLD_WORD_PATTERN.matches_word(word))
    {
        let mut idx = 1usize;
        let mut count = 1u32;
        if let Some((value, used)) = parse_number(&clause_tokens[idx..]) {
            count = value;
            idx += used;
        } else if clause_words
            .get(idx)
            .is_some_and(|word| CLAUSE_ARTICLE_WORD_PATTERN.matches_word(word))
        {
            idx += 1;
        }

        let subtype_word = clause_words.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing subtype in behold clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| strip_suffix_char(subtype_word, 's').and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported subtype in behold clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        if idx + 1 != clause_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing behold clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(EffectAst::subject_verb_behold(subtype, count)));
    }

    if CLAUSE_BLIGHT_WORD_PATTERN.matches_first_word(&clause_words) {
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for blight clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing blight clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_put_counters(
            crate::object::CounterType::MinusOneMinusOne,
            Value::Fixed(amount as i32)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::BlightKeywordAction),
            TargetAst::Object(ObjectFilter::creature().you_control(), None, None),
            None,
            false,
        )));
    }

    if CLAUSE_MANIFEST_DREAD_PREFIX_PATTERN.matches_words(&clause_words) {
        let manifest_dread = EffectAst::subject_verb_manifest_dread(PlayerAst::Implicit);
        let trailing_words = &clause_words[2..];
        if trailing_words.is_empty() {
            return Ok(Some(manifest_dread));
        }

        if CLAUSE_TWICE_WORD_PATTERN.matches_words(trailing_words) {
            return Ok(Some(EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![manifest_dread],
            }));
        }

        if trailing_words
            .last()
            .is_some_and(|word| CLAUSE_TIME_OR_TIMES_WORD_PATTERN.matches_word(word))
        {
            let value_tokens = &clause_tokens[2..clause_tokens.len() - 1];
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing manifest dread count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported manifest dread count tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![manifest_dread],
            }));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing manifest dread clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if CLAUSE_MANIFEST_TOP_YOUR_LIBRARY_PATTERN.matches_words(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_manifest_top_card(
            PlayerAst::You,
        )));
    }

    if CLAUSE_MANIFEST_CARD_FROM_HAND_PATTERN.matches_words(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_manifest_from_hand(
            PlayerAst::You,
        )));
    }

    if CLAUSE_MANIFEST_TOP_THAT_PLAYER_LIBRARY_PATTERN.matches_words(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_manifest_top_card(
            PlayerAst::ThatPlayerOrTargetController,
        )));
    }

    if CLAUSE_ITS_CONTROLLER_MANIFESTS_TOP_PATTERN.matches_words(&clause_words) {
        return Ok(Some(EffectAst::subject_verb_manifest_top_card(
            PlayerAst::ThatPlayerOrTargetController,
        )));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_POPULATE_WORD_PATTERN.matches_word(word))
    {
        if clause_words.len() == 1 {
            return Ok(Some(EffectAst::subject_verb_populate(Value::Fixed(1))));
        }

        if clause_words
            .get(1)
            .is_some_and(|word| CLAUSE_TWICE_WORD_PATTERN.matches_word(word))
            && clause_words.len() == 2
        {
            return Ok(Some(EffectAst::subject_verb_populate(Value::Fixed(2))));
        }

        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for populate clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let trailing = &clause_words[1 + used..];
        if !CLAUSE_TIME_OR_TIMES_WORD_PATTERN.matches_words(trailing) {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing populate clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(EffectAst::subject_verb_populate(count)));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_MELD_WORD_PATTERN.matches_word(word))
        && let Some(into_idx) = word_slice_find_word_where(&clause_words, |word| {
            CLAUSE_INTO_WORD_PATTERN.matches_word(word)
        })
    {
        let subject_words = &clause_words[1..into_idx];
        if !CLAUSE_MELD_SUBJECT_PATTERN.matches_words(subject_words) {
            return Err(CardTextError::ParseError(format!(
                "unsupported meld subject (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if into_idx + 1 >= clause_words.len() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let result_name = clause_words[into_idx + 1..].join(" ");
        return Ok(Some(EffectAst::subject_verb_meld(
            result_name,
            false,
            false,
        )));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_BOLSTER_SUPPORT_ADAPT_WORD_PATTERN.matches_word(word))
    {
        let keyword = clause_words[0];
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effect = match keyword {
            "bolster" => EffectAst::subject_verb_bolster(amount),
            "support" => EffectAst::subject_verb_support(amount),
            "adapt" => EffectAst::subject_verb_adapt(amount),
            _ => unreachable!(),
        };
        return Ok(Some(effect));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_FATESEAL_WORD_PATTERN.matches_word(word))
    {
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for fateseal clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing fateseal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_fateseal(
            PlayerAst::You,
            count,
        )));
    }

    if clause_words
        .first()
        .is_some_and(|word| CLAUSE_DISCOVER_WORD_PATTERN.matches_word(word))
    {
        if clause_words
            .get(1..)
            .is_some_and(|tail| CLAUSE_DISCOVER_AGAIN_SAME_VALUE_TAIL_PATTERN.matches_words(tail))
        {
            return Ok(Some(EffectAst::subject_verb_discover(
                PlayerAst::You,
                Value::EventValue(EventValueSpec::Amount),
            )));
        }
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for discover clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discover clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_discover(
            PlayerAst::You,
            count,
        )));
    }

    if let Some(explore_idx) = clause_words
        .iter()
        .position(|word| CLAUSE_EXPLORE_WORD_PATTERN.matches_word(word))
    {
        let tail_words = &clause_words[explore_idx + 1..];
        if !tail_words.is_empty()
            && !CLAUSE_AGAIN_WORD_PATTERN.matches_words(tail_words)
            && !tail_words
                .last()
                .is_some_and(|word| CLAUSE_TIME_OR_TIMES_WORD_PATTERN.matches_word(word))
        {
            return Ok(None);
        }

        let subject_tokens = &clause_tokens[..explore_idx];
        let subject_word_view = ClausePatternCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let target = if subject_words.is_empty()
            || CLAUSE_SOURCE_SUBJECT_WORDS_PATTERN.matches_words(&subject_words)
        {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        let explore = EffectAst::subject_verb_explore(target);
        if tail_words
            .last()
            .is_some_and(|word| CLAUSE_TIME_OR_TIMES_WORD_PATTERN.matches_word(word))
        {
            let value_tokens = &clause_tokens[explore_idx + 1..clause_tokens.len() - 1];
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing explore count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported explore count tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![explore],
            }));
        }
        return Ok(Some(explore));
    }

    if let Some(endure_idx) = clause_words
        .iter()
        .position(|word| matches!(*word, "endure" | "endures"))
    {
        let amount_tokens = trim_commas(&clause_tokens[endure_idx + 1..]);
        let (amount, used) = parse_value(&amount_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing endure count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if used != amount_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported endure count tail (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let subject_tokens = &clause_tokens[..endure_idx];
        let subject_word_view = ClausePatternCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let target = if subject_words.is_empty()
            || word_slice_eq_any(
                &subject_words,
                &[
                    &["it"],
                    &["this"],
                    &["this", "creature"],
                    &["this", "permanent"],
                ],
            ) {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        return Ok(Some(EffectAst::subject_verb_endure(target, amount)));
    }

    Ok(None)
}

pub(crate) fn parse_connive_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(connive_idx) = find_token_index_rev(tokens, |token| {
        CLAUSE_CONNIVE_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let mut count = Value::Fixed(1);
    let mut trailing_tokens = trim_commas(&tokens[connive_idx + 1..]);
    if !trailing_tokens.is_empty() {
        let Some((parsed_count, used)) = parse_value(&trailing_tokens) else {
            return Ok(None);
        };
        count = parsed_count;
        trailing_tokens = trim_commas(&trailing_tokens[used..]);
        if !trailing_tokens.is_empty() {
            let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                return Ok(None);
            };
            count = super::super::util::replace_unbound_x_with_value(
                count,
                &where_value,
                &crate::runtime_backend::token_word_refs(&trailing_tokens).join(" "),
            )?;
        }
    }

    if trailing_tokens
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..connive_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_word_view = ClausePatternCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if CLAUSE_CONVOKED_THIS_SPELL_SUBJECT_PATTERN.matches_words(&subject_words) {
        return Ok(Some(EffectAst::ForEachTagged {
            tag: TagKey::from("convoked_this_spell"),
            effects: vec![EffectAst::subject_verb_connive_iterated()],
        }));
    }

    let target_tokens = if subject_words.len() >= 4
        && CLAUSE_EACH_OF_X_TARGET_PREFIX_PATTERN.matches_words(&subject_words)
    {
        &subject_tokens[2..]
    } else {
        subject_tokens
    };
    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(EffectAst::subject_verb_connive(target, count)))
}
