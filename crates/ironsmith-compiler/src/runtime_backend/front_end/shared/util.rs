use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::TextSpan;
use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CardTextError, IT_TAG, KeywordAction, ParsedAbility, PlayerAst,
    ReferenceImports, TargetAst,
};
use crate::cost::OptionalCost;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, EventValueSpec, Value};
use crate::filter::AlternativeCastKind;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::target::{
    ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter, SourceReferenceSurface,
    TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;
use crate::{ChoiceCount, PowerToughness, PtValue, TagKey};

use super::activation_and_restrictions::activated_line_core::parse_activation_cost;
use super::activation_and_restrictions::keyword_action_costs::parse_ability_phrase;
use super::clause_support::parse_effect_sentences_lexed;
use super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::effect_sentences::find_verb;
use super::grammar::primitives::{split_lexed_slices_on_or, token_slice_span};
use super::keyword_static::keyword_action_to_static_ability;
use super::keyword_static::parse_this_spell_cost_condition;
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, contains_token_word_sequence, find_token_kind,
    find_token_word, lex_line, parser_token_word_refs, render_token_slice, token_slice_at_is,
    token_slice_at_is_any, token_slice_first_is, token_slice_first_is_any, token_slice_first_kind,
    token_slice_starts_with, token_word_refs, word_slice_at_is, word_slice_at_is_any,
    word_slice_contains_any_phrase, word_slice_contains_phrase, word_slice_contains_word,
    word_slice_ends_with, word_slice_ends_with_any, word_slice_eq, word_slice_eq_any,
    word_slice_find_window_by, word_slice_find_word, word_slice_first_is, word_slice_first_is_any,
    word_slice_last_is_any, word_slice_starts_with, word_slice_starts_with_any,
};
use super::object_filters::parse_object_filter;
use super::token_primitives::{
    self as shared_tokens, find_index, find_window_by, iter_eq, slice_contains, slice_ends_with,
    slice_starts_with, str_contains, str_contains_char, str_find, str_split_once,
    str_split_once_char, str_starts_with, str_starts_with_char, str_strip_prefix, str_strip_suffix,
    str_strip_suffix_char,
};
use std::cell::RefCell;
use std::collections::HashMap;

const SACRIFICE_COST_TAG_PREFIX: &str = "sacrifice_cost_";
const EXILE_COST_TAG_PREFIX: &str = "exile_cost_";
const UNATTACH_COST_TAG_PREFIX: &str = "unattach_cost_";

#[derive(Clone)]
struct SourceReferenceAlias {
    words: Vec<String>,
    surface: SourceReferenceSurface,
}

#[derive(Clone, Default)]
struct SourceReferenceContext {
    source_name: String,
    aliases: Vec<SourceReferenceAlias>,
    surfaces_by_span: HashMap<TextSpan, SourceReferenceSurface>,
}

thread_local! {
    static SOURCE_REFERENCE_CONTEXT: RefCell<SourceReferenceContext> =
        RefCell::new(SourceReferenceContext::default());
}

const CAST_THIS_SPELL_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["cast", "this", "spell", "only"]);
const LEVEL_UP_PREFIX_WORDS: &[&str] = &["level", "up"];
const MANA_VALUE_WORDS: &[&str] = &["mana", "value"];
const CAST_ONLY_NO_PERMANENTS_NAMED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if", "no", "permanents", "named"]; suffix & ["are", "on", "the", "battlefield"]);
const CAST_ONLY_DECLARE_ATTACKERS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["during", "the", "declare", "attackers", "step"],
            &["during", "declare", "attackers", "step"],
        ]
);
const CAST_ONLY_DECLARE_ATTACKERS_IF_ATTACKED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "during",
                "the",
                "declare",
                "attackers",
                "step",
                "and",
                "only",
                "if",
                "youve",
                "been",
                "attacked",
                "this",
                "step",
            ],
            &[
                "during",
                "declare",
                "attackers",
                "step",
                "and",
                "only",
                "if",
                "youve",
                "been",
                "attacked",
                "this",
                "step",
            ],
        ]
);
const CAST_ONLY_DURING_COMBAT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["during", "combat"]);
const CAST_ONLY_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["during", "combat", "before", "blockers", "are", "declared"]);
const CAST_ONLY_COMBAT_AFTER_BLOCKERS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["during", "combat", "after", "blockers", "are", "declared"]);
const CAST_ONLY_YOUR_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "during", "combat", "on", "your", "turn", "before", "blockers", "are", "declared",
        ]
);
const CAST_ONLY_OPPONENT_COMBAT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["during", "combat", "on", "an", "opponents", "turn"]);
const CAST_ONLY_BEFORE_ATTACKERS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["before", "attackers", "are", "declared"]);
const CAST_ONLY_BEFORE_COMBAT_DAMAGE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["before", "the", "combat", "damage", "step"],
            &["before", "combat", "damage", "step"]
        ]
);
const CAST_ONLY_OPPONENTS_UPKEEP_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["during", "an", "opponents", "upkeep"],
            &["during", "opponents", "upkeep"]
        ]
);
const CAST_ONLY_OPPONENT_TURN_AFTER_UPKEEP_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "during",
            "an",
            "opponents",
            "turn",
            "after",
            "their",
            "upkeep",
            "step",
        ]
);
const CAST_ONLY_YOUR_END_STEP_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["during", "your", "end", "step"]);
const CAST_ONLY_CAST_ANOTHER_SPELL_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["if", "youve", "cast", "another", "spell", "this", "turn"]);
const CAST_ONLY_CAST_ANOTHER_GREEN_SPELL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if", "youve", "cast", "another", "green", "spell", "this", "turn",
        ]
);
const CAST_ONLY_OPPONENT_CAST_CREATURE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if", "an", "opponent", "cast", "a", "creature", "spell", "this", "turn",
        ]
);
const CAST_ONLY_CREATURE_ATTACKING_YOU_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["if", "a", "creature", "is", "attacking", "you"]);
const CAST_ONLY_AFTER_COMBAT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["after", "combat"]);
const CAST_ONLY_CONTROL_SNOW_LAND_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["if", "you", "control", "a", "snow", "land"]);
const CAST_ONLY_FEWER_CREATURES_THAN_EACH_OPPONENT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if",
            "you",
            "control",
            "fewer",
            "creatures",
            "than",
            "each",
            "opponent",
        ]
);
const CAST_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "control"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if"]);
const FREERUNNING_ASSASSIN_OR_COMMANDER_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ]
);
const DEALT_DAMAGE_BY_CREATURES_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["youve", "been", "dealt", "damage", "by"],
            &["you", "have", "been", "dealt", "damage", "by"],
        ];
    suffix & ["creatures", "this", "turn"]
);
const YOUVE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["youve"]);
const SELF_FREE_CAST_ALTERNATIVE_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
            ],
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ],
        ]
);
const FLASH_WITH_ADDITIONAL_COST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash", "if",
            "you", "pay",
        ]
);
const FLASH_WITH_ADDITIONAL_COST_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["more", "to", "cast", "it"]);
const RATHER_THAN_THIS_SPELL_COST_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["than", "pay", "this"];
    contains_words & ["mana", "cost"];
    contains_any_words & [&["spell", "spells"]]
);
const RATHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["rather"]);
const COST_OR_COSTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost"], &["costs"]]);

pub(crate) fn with_source_reference_context<T>(card_name: &str, f: impl FnOnce() -> T) -> T {
    let aliases = source_reference_aliases_for_name(card_name);
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let previous = context.replace(SourceReferenceContext {
            source_name: card_name.trim().to_string(),
            aliases,
            surfaces_by_span: HashMap::new(),
        });
        let result = f();
        context.replace(previous);
        result
    })
}

pub(crate) fn current_source_reference_name() -> Option<String> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let source_name = context.borrow().source_name.trim().to_string();
        (!source_name.is_empty()).then_some(source_name)
    })
}

pub(crate) fn source_reference_surface_for_span(
    span: Option<TextSpan>,
) -> Option<SourceReferenceSurface> {
    let span = span?;
    SOURCE_REFERENCE_CONTEXT.with(|context| context.borrow().surfaces_by_span.get(&span).cloned())
}

pub(crate) fn record_source_reference_surface(
    span: Option<TextSpan>,
    surface: SourceReferenceSurface,
) {
    let Some(span) = span else {
        return;
    };
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        context.borrow_mut().surfaces_by_span.insert(span, surface);
    });
}

fn source_reference_aliases_for_name(name: &str) -> Vec<SourceReferenceAlias> {
    let mut aliases = Vec::new();
    let mut push_alias = |raw: &str, surface: SourceReferenceSurface| {
        for words in source_reference_word_variants_from_text(raw) {
            if !words.is_empty()
                && !aliases
                    .iter()
                    .any(|alias: &SourceReferenceAlias| alias.words == words)
            {
                aliases.push(SourceReferenceAlias {
                    words,
                    surface: surface.clone(),
                });
            }
        }
    };

    let trimmed = name.trim();
    if trimmed.is_empty() {
        return aliases;
    }

    let mut full_names = Vec::new();
    push_unique_source_name_alias(&mut full_names, trimmed);
    if let Some((front_face, _)) = str_split_once(trimmed, "//") {
        push_unique_source_name_alias(&mut full_names, front_face);
    }
    let existing_full_names = full_names.clone();
    for full_name in existing_full_names {
        if let Some(stripped) = strip_leading_digital_variant_marker(full_name.as_str()) {
            push_unique_source_name_alias(&mut full_names, stripped);
        }
        if let Some(stripped) = strip_trailing_roman_numeral(full_name.as_str()) {
            push_unique_source_name_alias(&mut full_names, stripped);
        }
    }

    for full_name in &full_names {
        push_alias(
            full_name,
            SourceReferenceSurface::FullName(full_name.to_string()),
        );
        for article in ["The ", "A ", "An "] {
            if let Some(rest) = str_strip_prefix(full_name.as_str(), article) {
                let rest = rest.trim();
                if !rest.is_empty() {
                    push_alias(
                        rest,
                        SourceReferenceSurface::FullName(full_name.to_string()),
                    );
                }
            }
        }
    }

    for full_name in &full_names {
        if let Some((short_name, _)) = str_split_once_char(full_name.as_str(), ',') {
            let short_name = short_name.trim();
            push_alias(
                short_name,
                SourceReferenceSurface::ShortName(short_name.to_string()),
            );
            if let Some(rest) = strip_leading_digital_variant_marker(short_name) {
                push_alias(rest, SourceReferenceSurface::ShortName(rest.to_string()));
            }
        } else if let Some(rest) = strip_leading_digital_variant_marker(full_name) {
            let rest = rest.trim();
            if !rest.is_empty() {
                push_alias(rest, SourceReferenceSurface::ShortName(rest.to_string()));
            }
        } else if let Some((short_name, _)) = str_split_once_char(full_name.as_str(), ' ') {
            let short_name = short_name.trim();
            let lower_short_name = short_name.to_ascii_lowercase();
            if !matches!(lower_short_name.as_str(), "a" | "an" | "the")
                && parse_card_type(&lower_short_name).is_none()
                && parse_subtype_word(&lower_short_name).is_none()
            {
                push_alias(
                    short_name,
                    SourceReferenceSurface::ShortName(short_name.to_string()),
                );
            }
        }
    }
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.words.len()));
    aliases
}

fn push_unique_source_name_alias(aliases: &mut Vec<String>, raw: &str) {
    let raw = raw.trim();
    if !raw.is_empty() && !aliases.iter().any(|existing| existing == raw) {
        aliases.push(raw.to_string());
    }
}

fn strip_leading_digital_variant_marker(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
        let rest = trimmed[2..].trim();
        if !rest.is_empty() {
            return Some(rest);
        }
    }
    None
}

fn strip_trailing_roman_numeral(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    let (prefix, suffix) = trimmed.rsplit_once(char::is_whitespace)?;
    let suffix = suffix.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
    if suffix.len() < 2
        || !suffix.bytes().all(|byte| {
            matches!(
                byte.to_ascii_uppercase(),
                b'I' | b'V' | b'X' | b'L' | b'C' | b'D' | b'M'
            )
        })
    {
        return None;
    }
    let prefix = prefix.trim();
    (!prefix.is_empty()).then_some(prefix)
}

fn source_reference_words_from_text(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        let normalized = match ch {
            '’' | '‘' => '\'',
            '−' => '-',
            _ => ch,
        };
        if normalized.is_ascii_alphanumeric() {
            current.push(normalized.to_ascii_lowercase());
        } else if matches!(normalized, '\'') {
            continue;
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn source_reference_word_variants_from_text(text: &str) -> Vec<Vec<String>> {
    let parser_words = source_reference_words_from_text(text);
    let token_words = text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '\'' || ch == '’' || ch == '-'))
        .filter(|word| !word.is_empty())
        .map(|word| {
            word.to_ascii_lowercase()
                .replace('’', "'")
                .replace('‘', "'")
                .replace('−', "-")
        })
        .collect::<Vec<_>>();
    let mut variants = vec![parser_words.clone()];
    if token_words != parser_words {
        variants.push(token_words);
    }
    let article_stripped = parser_words
        .iter()
        .filter(|word| !is_article(word))
        .cloned()
        .collect::<Vec<_>>();
    if !article_stripped.is_empty()
        && !variants.iter().any(|variant| {
            iter_eq(
                variant.iter().map(String::as_str),
                article_stripped.iter().map(String::as_str),
            )
        })
    {
        variants.push(article_stripped);
    }
    variants
}

pub(crate) fn source_reference_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        context
            .borrow()
            .aliases
            .iter()
            .find(|alias| {
                alias.words.len() == words.len()
                    && iter_eq(
                        alias.words.iter().map(String::as_str),
                        words.iter().copied(),
                    )
            })
            .map(|alias| alias.surface.clone())
    })
}

pub(crate) fn source_reference_surface_for_possessive_words(
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        context
            .borrow()
            .aliases
            .iter()
            .find(|alias| source_reference_words_match_possessive(&alias.words, words))
            .map(|alias| alias.surface.clone())
    })
}

fn source_reference_words_match_possessive(alias_words: &[String], words: &[&str]) -> bool {
    if alias_words.len() != words.len() || alias_words.is_empty() {
        return false;
    }

    let Some((alias_last, alias_prefix)) = alias_words.split_last() else {
        return false;
    };
    let Some((word_last, word_prefix)) = words.split_last() else {
        return false;
    };
    let possessive_last = format!("{alias_last}s");
    iter_eq(
        alias_prefix.iter().map(String::as_str),
        word_prefix.iter().copied(),
    ) && (*word_last == alias_last.as_str() || *word_last == possessive_last)
}

pub(crate) fn source_choose_spec_for_surface(surface: SourceReferenceSurface) -> ChooseSpec {
    ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface))
}

pub(crate) fn this_source_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    if !is_this_source_reference_words(words) {
        return None;
    }
    Some(SourceReferenceSurface::ThisPermanentType(words.join(" ")))
}

#[cfg(test)]
pub(crate) fn tokenize_line(line: &str, line_index: usize) -> Vec<OwnedLexToken> {
    let mut tokens = lex_line(line, line_index).expect("test tokenization helper should lex input");
    for token in &mut tokens {
        token.lowercase_word();
    }
    tokens
}

pub(crate) use super::lexer::parser_token_word_refs as words;

type UtilWordView<'a> = TokenWordView<'a>;

const FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["for", "each"]);
const EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["each"]);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["other"], &["another"]]);
const PLUS_OR_MINUS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["plus"], &["minus"]]);
const MINUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["minus"]);
const BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["basic", "land", "type", "among"],
            &["basic", "land", "types", "among"],
        ]
);
const CREATURE_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "type", "among"],
            &["creature", "types", "among"],
        ]
);
const COLORS_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["color", "among"], &["colors", "among"]]);
const DIFFERENT_POWERS_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["different", "powers", "among"],
            &["different", "power", "values", "among"],
            &["different", "power", "among"],
        ]
);
const SPELL_CAST_THIS_TURN_COUNT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["this", "turn"];
    contains_any_words & [&["spell", "spells"], &["cast", "casts"]]
);
const OTHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["other"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const OR_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const IT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const COPY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["copy"]);
const EXCEPT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["except"]);
const TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const PARSE_TARGET_LEADING_CONDITION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["during"], &["if"], &["until"]]);
const PARSE_TARGET_SPLIT_PREFIX_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["during"], &["if"], &["then"], &["until"]]);
const CHOSEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["chosen"]);
const INSTEAD_THIS_WAY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["instead"], &["this"], &["way"]]);
const ATTACKING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attacking"]);
const IT_THAT_ATTACKING_REFERENCE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["its", "it", "thats", "that"]]);
const MIXED_PLAYER_PLANESWALKER_TOKEN_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["player", "planeswalker", "token"]);
const IT_OR_THEM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const WHO_OR_THAT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["who"], &["that"]]);
const AT_LEAST_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at", "least"]);
const MORE_CARD_IN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["more", "card", "in"], &["more", "cards", "in"]]);
const THEIR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["their"]);
const HAND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["hand"]);
const THAN_YOU_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than", "you"]);
const DO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["do"]);
const AS_YOU_ACTIVATE_THIS_ABILITY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["as", "you", "activate", "this", "ability"]);
const MORE_LIFE_THAN_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["more", "life", "than", "you"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const UNTIL_END_OF_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["until", "end", "of", "turn"]);
const UNTIL_END_OF_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "end", "of", "turn"]);
const THIS_OR_THISS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["thiss"]]);
const OF_WORD_EXACT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const SOURCE_REFERENCE_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["source"],
            &["spell"],
            &["permanent"],
            &["card"],
            &["creature"]
        ]
);
const OUTLAW_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["outlaw"], &["outlaws"]]);
const NON_OUTLAW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["nonoutlaw"],
            &["nonoutlaws"],
            &["non-outlaw"],
            &["non-outlaws"],
        ]
);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const STRIKE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["strike"]);
const ANOTHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["another"]);
const MANA_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["mana", "ability"], &["mana", "abilities"]]);
const BASIC_LANDCYCLING_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["basic", "landcycling"]);
const CYCLING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cycling"]);
const CYCLING_SUFFIX_CHARS: &[char] = &['c', 'y', 'c', 'l', 'i', 'n', 'g'];
const BARGAIN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["bargain"]);
const REPLICATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["replicate"]);
const ESCAPE_EXILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exile"]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const FROM_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "your", "graveyard"]);
const FLASHBACK_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["flashback"]);
const JUMP_START_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["jumpstart"], &["jump-start"]]);
const JUMP_START_SPLIT_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["jump", "start"]);
const PAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pay"]);
const LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const TRANSMUTE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["transmute"]);
const REINFORCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["reinforce"]);
const NO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["no"]);
const EXACTLY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exactly"]);
const FEWER_OR_LESS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["fewer"], &["less"]]);
const MORE_OR_GREATER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["more"], &["greater"]]);
const THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const AT_LEAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["at", "least"]);
const AT_MOST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["at", "most"]);
const QUANTITY_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"]]);
const ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const ONE_OR_MORE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one", "or", "more"]);
const AND_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["and"], &["or"]]);
const OR_MORE_COUNTER_DESCRIPTOR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["or"], &["more"]]);
const THAT_PLAYER_OR_THAT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player", "or", "that"]);
const THAT_OR_THE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["the"]]);
const CONTROLLED_OBJECT_PLURAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creatures"],
            &["permanents"],
            &["planeswalkers"],
            &["sources"],
            &["spells"],
        ]
);
const CONTROLLER_OR_OWNER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["controller"], &["owner"]]);
const CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["controller"], &["controllers"], &["owner"], &["owners"],]);
const CONTROLLER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["controller"]);
const OWNER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["owner"]);
const ANY_NUMBER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["any", "number"]);
const THEN_OR_AND_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["then"], &["and"]]);
const MOST_CARDS_IN_HAND_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "the", "player", "who", "has", "the", "most", "cards", "in", "hand"
            ],
            &["player", "who", "has", "the", "most", "cards", "in", "hand"],
            &[
                "the", "player", "with", "the", "most", "cards", "in", "hand"
            ],
            &["player", "with", "the", "most", "cards", "in", "hand"],
        ]
);
const MOST_LIFE_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "player", "who", "has", "the", "most", "life"],
            &["player", "who", "has", "the", "most", "life"],
            &["the", "player", "with", "the", "most", "life"],
            &["player", "with", "the", "most", "life"],
        ]
);
const LOWEST_LIFE_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "the", "player", "who", "has", "the", "lowest", "life", "total"
            ],
            &["player", "who", "has", "the", "lowest", "life", "total"],
            &["the", "player", "with", "the", "lowest", "life", "total"],
            &["player", "with", "the", "lowest", "life", "total"],
        ]
);
const HAS_OR_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has"], &["have"]]);
const YOU_OR_YOUR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you"], &["your"]]);
const TARGET_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["target", "opponent"], &["target", "opponents"]]);
const TARGET_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["target", "player"], &["target", "players"]]);
const PLAYER_OF_YOUR_CHOICE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["a", "player", "of", "your", "choice"],
            &["player", "of", "your", "choice"]
        ]
);
const OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"], &["an", "opponent"]]);
const OPPONENT_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);
const LIBRARY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["library"]);
const YOUR_OPPONENTS_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["your", "opponents"], &["opponents"]]);
const DEFENDING_PLAYER_CHOICE_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["defending", "player", "choice"]);
const CHOSEN_AT_RANDOM_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["chosen", "at", "random"]);
const AT_RANDOM_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["at", "random"]);
const ANY_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["any"], &["any", "target"], &["any", "targets"]]);
const ANY_OTHER_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["any", "other"],
            &["any", "other", "target"],
            &["any", "other", "targets"],
            &["other"],
            &["the", "other"],
        ]
);
const UP_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["up", "to"]);
const TARGET_OR_TARGETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const OTHER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["other", "target"], &["other", "targets"]]);
const TOP_CARD_TARGET_SHORTHAND_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["top", "card"], &["card"]]);
const CARDS_TARGET_SHORTHAND_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cards"]);
const TARGET_COUNT_SELECTOR_MODIFIER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["tapped"],
            &["untapped"],
            &["attacking"],
            &["nonattacking"],
            &["blocked"],
            &["unblocked"],
            &["blocking"],
            &["nonblocking"],
            &["non"],
            &["other"],
            &["another"],
            &["nonartifact"],
            &["noncreature"],
            &["nonland"],
            &["nontoken"],
            &["legendary"],
            &["basic"],
        ]
);
const TARGET_COUNT_OBJECT_SELECTOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card"],
            &["cards"],
            &["permanent"],
            &["permanents"],
            &["creature"],
            &["creatures"],
            &["spell"],
            &["spells"],
            &["source"],
            &["sources"],
            &["token"],
            &["tokens"],
        ]
);
const OF_THOSE_OR_THEM_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["of", "those"], &["of", "them"]]);
const IT_OR_THEM_WITH_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["it", "with"], &["them", "with"]]);
const TAGGED_OBJECT_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "permanent"], &["that", "creature"], &["it"]]);
const REST_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["rest"],
            &["the", "rest"],
            &["rest", "of", "revealed", "cards"],
            &["the", "rest", "of", "revealed", "cards"],
        ]
);
const EQUIPPED_OBJECT_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["equipped", "creature"], &["equipped", "permanent"]]);
const ENCHANTED_OBJECT_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted", "creature"], &["enchanted", "permanent"]]);
const CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["creature", "tapped", "to", "pay", "this"];
    suffix_any
        & [
            &["additional", "cost"],
            &["additional", "costs"],
        ];
    contains_any_words & [&["spell", "spell's", "spell’s", "spells"]]
);
const ANY_PLAYER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["player"], &["players"]]);
const PLAYER_ON_YOUR_TEAM_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player", "on", "your", "team"],
            &["players", "on", "your", "team"]
        ]
);
const ENCHANTED_PLAYER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted", "player"], &["enchanted", "players"]]);
const THAT_PLAYER_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that", "player"]);
const CHOSEN_PLAYER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["chosen", "player"], &["chosen", "players"]]);
const THAT_OPPONENT_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "opponent"]);
const ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["one", "of", "your", "opponents"],
            &["one", "of", "your", "opponent"]
        ]
);
const SPELL_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const TRIGGERING_SPELL_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "spell"], &["those", "spells"]]);
const TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "spell", "or", "ability"],
            &["that", "ability", "or", "spell"]
        ]
);
const ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "controller"],
            &["its", "controllers"],
            &["their", "controller"],
            &["their", "controllers"],
        ]
);
const ITS_OR_THEIR_OWNER_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "owner"],
            &["its", "owners"],
            &["their", "owner"],
            &["their", "owners"],
        ]
);
const SOURCE_PT_REFERENCE_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["thiss", "power"],
            &["this", "power"],
            &["thiss", "toughness"],
            &["this", "toughness"],
            &["thiss", "base", "power", "and", "toughness"],
            &["this", "base", "power", "and", "toughness"],
        ]
);
const SOURCE_PT_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["thiss", "power", "and", "toughness"],
            &["this", "power", "and", "toughness"]
        ]
);
const IT_INSTEAD_THIS_WAY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it"]);
const TOKEN_CREATED_THIS_WAY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["token", "created", "this", "way"],
            &["tokens", "created", "this", "way"],
            &["that", "token", "created", "this", "way"],
            &["those", "tokens", "created", "this", "way"],
        ]
);
const ITSELF_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["itself"]);
const HIM_OR_HER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["him"], &["her"]]);
const THEM_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["them"]);
const OTHER_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["other", "player"], &["other", "players"]]);
const DEFENDING_PLAYER_EDGE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "player"]);
const DEFENDING_PLAYER_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["defending", "player"]);
const ATTACKING_PLAYER_EDGE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["attacking", "player"], &["the", "attacking", "player"]]);
const ATTACKING_PLAYER_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["attacking", "player"]);
const THAT_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["they"], &["that", "player"], &["the", "player"]]);
const VOTER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["the", "voter"], &["voter"]]);
const CHOSEN_PLAYER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "chosen", "player"],
            &["chosen", "player"],
            &["the", "chosen", "players"],
            &["chosen", "players"],
        ]
);
const THAT_PLAYERS_OR_THEIR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["that", "players"], &["their"]]);
const OWNERS_OF_THOSE_OBJECTS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "owners", "of", "those", "cards"],
            &["owners", "of", "those", "cards"],
            &["the", "owners", "of", "those", "objects"],
            &["owners", "of", "those", "objects"],
        ]
);
const ITS_CONTROLLER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["its", "controller"]);
const ITS_OR_THEIR_OWNER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["its", "owner"], &["their", "owner"]]);
const THIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this"]);
const ITS_OR_THEIR_CONTROLLER_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["its", "controller"], &["their", "controller"]]);
const ITS_OR_THEIR_OWNER_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["its", "owner"], &["their", "owner"]]);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const ONE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one"]);
const THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "way"]);
const TARGET_PLAYER_SPEED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "players", "speed"],
            &["target", "player", "speed"],
            &["that", "players", "speed"],
            &["that", "player", "speed"],
        ]
);
const SOURCE_POWER_SHORT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["its", "power"], &["this", "power"], &["thiss", "power"]]);
const SOURCE_POWER_LONG_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "power"],
            &["thiss", "creature", "power"],
            &["this", "creatures", "power"],
            &["thiss", "creatures", "power"],
        ]
);
const SOURCE_TOUGHNESS_SHORT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "toughness"],
            &["this", "toughness"],
            &["thiss", "toughness"]
        ]
);
const SOURCE_TOUGHNESS_LONG_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "toughness"],
            &["thiss", "creature", "toughness"],
            &["this", "creatures", "toughness"],
            &["thiss", "creatures", "toughness"],
        ]
);
const SOURCE_MANA_VALUE_SHORT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "mana", "value"],
            &["this", "mana", "value"],
            &["thiss", "mana", "value"],
        ]
);
const SOURCE_MANA_VALUE_LONG_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "mana", "value"],
            &["thiss", "creature", "mana", "value"],
            &["this", "creatures", "mana", "value"],
            &["thiss", "creatures", "mana", "value"],
        ]
);
const EXPLOITED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["exploited"]);
const NUMBER_OF_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["number", "of"]);
const SOURCE_COUNTER_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this", "artifact"],
            &["this", "land"],
            &["this", "enchantment"],
        ]
);
const TAGGED_COUNTER_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
        ]
);
const SOURCE_REGENERATED_THIS_TURN_COUNT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["time", "it", "regenerated", "this", "turn"],
            &["times", "it", "regenerated", "this", "turn"],
        ]
);
const YOU_DREW_CARDS_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you've", "drawn", "this", "turn"],
            &["cards", "you've", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
        ]
);
const OPPONENT_DREW_CARDS_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "an", "opponent", "has", "drawn", "this", "turn"],
            &["cards", "an", "opponent", "has", "drawn", "this", "turn"],
            &["card", "opponents", "have", "drawn", "this", "turn"],
            &["cards", "opponents", "have", "drawn", "this", "turn"],
        ]
);
const SOURCE_FROM_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "card", "from", "your", "graveyard"],
            &["thiss", "card", "from", "your", "graveyard"],
            &["this", "creature", "from", "your", "graveyard"],
            &["thiss", "creature", "from", "your", "graveyard"],
            &["this", "permanent", "from", "your", "graveyard"],
            &["thiss", "permanent", "from", "your", "graveyard"],
        ]
);
const SOURCE_FROM_YOUR_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [&["this"], &["thiss"]];
    contains_words & ["from", "your", "graveyard"];
    contains_any_words & [&["card", "creature", "permanent"]]
);
const SOURCE_FROM_YOUR_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "card", "from", "your", "hand"],
            &["thiss", "card", "from", "your", "hand"],
            &["this", "creature", "from", "your", "hand"],
            &["thiss", "creature", "from", "your", "hand"],
            &["this", "permanent", "from", "your", "hand"],
            &["thiss", "permanent", "from", "your", "hand"],
            &["this", "from", "your", "hand"],
            &["thiss", "from", "your", "hand"],
        ]
);
const FROM_COMMAND_ZONE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "command", "zone"]);
const DISCARD_THIS_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["discard", "this", "card"]]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const BEGINNING_OF_END_STEP_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["beginning", "of", "the", "next", "end", "step"],
            &["beginning", "of", "next", "end", "step"],
            &["beginning", "of", "the", "end", "step"],
            &["beginning", "of", "end", "step"],
        ]]
);
const SACRIFICE_DELAY_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["sacrifice"];
    contains_any_words & [&["token", "tokens", "permanent", "permanents", "it", "them"]]
);
const EXILE_DELAY_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["exile"];
    contains_any_words & [&["token", "tokens", "permanent", "permanents", "it", "them"]]
);
const X_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);
const VALUE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["value"]);
const EVENT_AMOUNT_VALUE_PATTERNS: &[(ClauseShape<'static>, usize)] = &[
    (
        clause_shape!(prefix_any & [&["that", "many"], &["that", "much"], &["that", "amount"]]),
        2,
    ),
    (
        clause_shape!(prefix & ["the", "amount", "of", "e", "paid", "this", "way"]),
        7,
    ),
    (
        clause_shape!(prefix & ["amount", "of", "e", "paid", "this", "way"]),
        6,
    ),
    (
        clause_shape!(prefix & ["that", "amount", "of", "excess", "damage"]),
        5,
    ),
    (
        clause_shape!(prefix & ["that", "much", "excess", "damage"]),
        4,
    ),
    (clause_shape!(prefix & ["damage", "dealt"]), 2),
    (clause_shape!(prefix & ["the", "damage", "dealt"]), 3),
    (clause_shape!(prefix & ["that", "damage"]), 2),
    (clause_shape!(prefix & ["the", "result"]), 2),
    (clause_shape!(prefix & ["that", "result"]), 2),
    (clause_shape!(prefix & ["result"]), 1),
];
const OTHER_RESULT_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "other", "result"]);
const NUMBER_OF_REMOVED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(prefix_any & [&["the", "number", "of"], &["number", "of"]]; suffix & ["removed", "this", "way"]);
const YOUR_SPEED_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["your", "speed"]);
const SPELL_CAST_THIS_TURN_SUFFIX_PATTERNS: &[(ClauseShape<'static>, usize, PlayerFilter)] = &[
    (
        clause_shape!(suffix & ["theyve", "cast", "this", "turn"]),
        4,
        PlayerFilter::IteratedPlayer,
    ),
    (
        clause_shape!(suffix & ["they", "cast", "this", "turn"]),
        4,
        PlayerFilter::IteratedPlayer,
    ),
    (
        clause_shape!(suffix & ["that", "player", "cast", "this", "turn"]),
        5,
        PlayerFilter::IteratedPlayer,
    ),
    (
        clause_shape!(suffix & ["youve", "cast", "this", "turn"]),
        4,
        PlayerFilter::You,
    ),
    (
        clause_shape!(suffix & ["you", "cast", "this", "turn"]),
        4,
        PlayerFilter::You,
    ),
    (
        clause_shape!(suffix & ["an", "opponent", "has", "cast", "this", "turn"]),
        6,
        PlayerFilter::Opponent,
    ),
    (
        clause_shape!(suffix & ["opponent", "has", "cast", "this", "turn"]),
        5,
        PlayerFilter::Opponent,
    ),
    (
        clause_shape!(suffix & ["opponents", "have", "cast", "this", "turn"]),
        5,
        PlayerFilter::Opponent,
    ),
    (
        clause_shape!(suffix & ["cast", "this", "turn"]),
        3,
        PlayerFilter::Any,
    ),
];

fn is_target_count_selector_modifier(word: &str) -> bool {
    TARGET_COUNT_SELECTOR_MODIFIER_WORD_PATTERN.matches_word(word)
}

fn is_target_count_object_selector(word: &str) -> bool {
    TARGET_COUNT_OBJECT_SELECTOR_WORD_PATTERN.matches_word(word)
        || parse_card_type(word).is_some()
        || parse_non_type(word).is_some()
        || parse_subtype_word(word).is_some()
        || str_strip_suffix(word, "s")
            .and_then(parse_subtype_word)
            .is_some()
}

fn target_count_object_selector_index(tokens: &[OwnedLexToken], start: usize) -> usize {
    let mut object_selector_idx = start;
    while tokens
        .get(object_selector_idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(is_target_count_selector_modifier)
    {
        object_selector_idx += 1;
    }
    object_selector_idx
}

fn title_case_card_name_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    let mut idx = if FOR_EACH_PREFIX_PATTERN.matches_words(words) {
        2
    } else if words
        .first()
        .is_some_and(|word| EACH_WORD_PATTERN.matches_word(word))
    {
        1
    } else {
        return None;
    };

    if words
        .get(idx)
        .is_some_and(|word| OF_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if idx >= words.len() {
        return None;
    }

    let mut other = false;
    if words
        .get(idx)
        .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word))
    {
        other = true;
        idx += 1;
    }
    if idx >= words.len() {
        return None;
    }

    let parse_scope_filter = |scope_start: usize| -> Option<(ObjectFilter, usize)> {
        let mut scope_end = scope_start;
        while scope_end < words.len() && !PLUS_OR_MINUS_WORD_PATTERN.matches_word(words[scope_end])
        {
            scope_end += 1;
        }
        if scope_end == scope_start {
            return None;
        }
        let scope_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[scope_start..scope_end]);
        let filter = parse_object_filter(&scope_tokens, false).ok()?;
        Some((filter, scope_end))
    };

    if BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&words[idx..]) {
        let mut scope_start = idx + 4;
        if words
            .get(scope_start)
            .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
        {
            scope_start += 1;
        }
        let (filter, used) = parse_scope_filter(scope_start)?;
        return Some((Value::BasicLandTypesAmong(filter), used));
    }

    if CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&words[idx..]) {
        let mut scope_start = idx + 3;
        if words
            .get(scope_start)
            .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
        {
            scope_start += 1;
        }
        let (filter, used) = parse_scope_filter(scope_start)?;
        return Some((Value::CreatureTypesAmong(filter), used));
    }

    if COLORS_AMONG_PREFIX_PATTERN.matches_words(&words[idx..]) {
        let mut scope_start = idx + 2;
        if words
            .get(scope_start)
            .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
        {
            scope_start += 1;
        }
        let (filter, used) = parse_scope_filter(scope_start)?;
        return Some((Value::ColorsAmong(filter), used));
    }

    let mut filter_end = idx;
    while filter_end < words.len() && !PLUS_OR_MINUS_WORD_PATTERN.matches_word(words[filter_end]) {
        filter_end += 1;
    }

    let this_way_start = THIS_WAY_PATTERN
        .find_exact_window(&words[idx..filter_end], 2)
        .map(|relative_idx| idx + relative_idx);
    if let Some(this_way_start) = this_way_start {
        for candidate_end in (idx + 1..this_way_start).rev() {
            let candidate_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(&words[idx..candidate_end]);
            if let Ok(filter) = parse_object_filter(&candidate_tokens, other) {
                return Some((
                    Value::Count(
                        filter.match_tagged(
                            TagKey::from(IT_TAG),
                            TaggedOpbjectRelation::IsTaggedObject,
                        ),
                    ),
                    filter_end,
                ));
            }
        }
    }

    let count_words = &words[idx..filter_end];
    if SOURCE_REGENERATED_THIS_TURN_COUNT_PATTERN.matches_words(count_words) {
        return Some((Value::SourceRegeneratedThisTurnCount, filter_end));
    }
    if YOU_DREW_CARDS_THIS_TURN_PATTERN.matches_words(count_words) {
        return Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), filter_end));
    }
    if OPPONENT_DREW_CARDS_THIS_TURN_PATTERN.matches_words(count_words) {
        return Some((
            Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            filter_end,
        ));
    }

    let filter_tokens =
        crate::runtime_backend::lexer::synthetic_word_tokens(&words[idx..filter_end]);
    let filter = parse_object_filter(&filter_tokens, other).ok()?;
    Some((Value::Count(filter), filter_end))
}

pub(crate) fn is_article(word: &str) -> bool {
    ARTICLE_WORD_PATTERN.matches_word(word)
}

pub(crate) fn strip_leading_word_refs_any<'slice, 'word>(
    mut words: &'slice [&'word str],
    leading_words: &[&str],
) -> &'slice [&'word str] {
    while words
        .first()
        .is_some_and(|word| leading_words.iter().any(|leading| word == leading))
    {
        words = &words[1..];
    }
    words
}

pub(crate) fn strip_leading_article_word_refs<'slice, 'word>(
    mut words: &'slice [&'word str],
) -> &'slice [&'word str] {
    while words.first().is_some_and(|word| is_article(word)) {
        words = &words[1..];
    }
    words
}

pub(crate) fn strip_leading_article_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let view = UtilWordView::new(tokens);
    let mut word_idx = 0usize;
    while view.get(word_idx).is_some_and(is_article) {
        word_idx += 1;
    }
    if word_idx == 0 {
        return tokens;
    }
    let token_idx = view
        .token_index_for_word_index(word_idx)
        .unwrap_or(tokens.len());
    &tokens[token_idx..]
}

pub(crate) fn strip_leading_token_words_any<'a>(
    tokens: &'a [OwnedLexToken],
    leading_words: &[&str],
) -> &'a [OwnedLexToken] {
    let view = UtilWordView::new(tokens);
    let mut word_idx = 0usize;
    while view
        .get(word_idx)
        .is_some_and(|word| leading_words.iter().any(|leading| word == *leading))
    {
        word_idx += 1;
    }
    if word_idx == 0 {
        return tokens;
    }
    let token_idx = view
        .token_index_for_word_index(word_idx)
        .unwrap_or(tokens.len());
    &tokens[token_idx..]
}

pub(crate) fn strip_leading_token_word_once_any<'a>(
    tokens: &'a [OwnedLexToken],
    leading_words: &[&str],
) -> (&'a [OwnedLexToken], bool) {
    if token_slice_first_is_any(tokens, leading_words) {
        (&tokens[1..], true)
    } else {
        (tokens, false)
    }
}

pub(crate) fn strip_leading_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    strip_leading_article_tokens(tokens).to_vec()
}

pub(crate) fn word_refs_at_is_article(words: &[&str], idx: usize) -> bool {
    words.get(idx).is_some_and(|word| is_article(word))
}

pub(crate) fn non_article_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect()
}

pub(crate) fn word_refs_except<'a>(words: &[&'a str], excluded: &[&str]) -> Vec<&'a str> {
    words
        .iter()
        .copied()
        .filter(|word| !excluded.iter().any(|excluded_word| word == excluded_word))
        .collect()
}

pub(crate) fn non_article_word_refs_except<'a>(
    words: &[&'a str],
    excluded: &[&str],
) -> Vec<&'a str> {
    let words = non_article_word_refs(words);
    word_refs_except(&words, excluded)
}

pub(crate) fn non_article_token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    let words = token_word_refs(tokens);
    non_article_word_refs(&words)
}

pub(crate) fn strip_possessive_suffix(word: &str) -> &str {
    str_strip_suffix(word, "'s")
        .or_else(|| str_strip_suffix(word, "’s"))
        .or_else(|| str_strip_suffix(word, "s'"))
        .or_else(|| str_strip_suffix(word, "s’"))
        .unwrap_or(word)
}

pub(crate) fn possessive_normalized_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .filter_map(|word| match *word {
            "s" | "'" | "’" => None,
            _ => Some(strip_possessive_suffix(word)),
        })
        .filter(|word| !word.is_empty())
        .collect()
}

pub(crate) fn non_article_possessive_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .filter_map(|word| match word {
            "s" | "'" | "’" => None,
            _ => Some(strip_possessive_suffix(word)),
        })
        .filter(|word| !word.is_empty())
        .collect()
}

const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper_";
const INSTEAD_WORD: &str = "instead";
const WOULD_WORD: &str = "would";
const THE_NEXT_TIME_PHRASE: &[&str] = &["the", "next", "time"];
const COUNTERED_THIS_WAY_PHRASE: &[&str] = &["countered", "this", "way"];
const INSTEAD_OF_PHRASE: &[&str] = &["instead", "of"];
const GRAVEYARD_WORD_PHRASE: &[&str] = &["graveyard"];
const INSTEAD_OF_PUTTING_IT_INTO_PHRASE: &[&str] = &["instead", "of", "putting", "it", "into"];
const INSTEAD_OF_PUTTING_THEM_INTO_PHRASE: &[&str] = &["instead", "of", "putting", "them", "into"];

pub(crate) fn helper_tag_for_tokens(tokens: &[OwnedLexToken], prefix: &str) -> TagKey {
    let span = span_from_tokens(tokens).unwrap_or(TextSpan {
        line: 0,
        start: 0,
        end: 0,
    });

    TagKey::from(format!(
        "{SENTENCE_HELPER_TAG_PREFIX}{prefix}_l{}_s{}_e{}",
        span.line, span.start, span.end
    ))
}

pub(crate) fn classify_instead_followup_text(
    text: &str,
) -> crate::cards::builders::InsteadSemantics {
    let Ok(tokens) = lex_line(text, 0) else {
        return crate::cards::builders::InsteadSemantics::NonReplacement;
    };
    classify_instead_followup_tokens(&tokens)
}

pub(crate) fn classify_instead_followup_tokens(
    tokens: &[OwnedLexToken],
) -> crate::cards::builders::InsteadSemantics {
    let words = token_word_refs(tokens);
    let first_instead = word_slice_find_word(&words, INSTEAD_WORD);
    if first_instead.is_none() {
        return crate::cards::builders::InsteadSemantics::NonReplacement;
    }

    let first_would = word_slice_find_word(&words, WOULD_WORD);
    if first_would.is_some_and(|would| first_instead.is_none_or(|instead| would < instead))
        || word_slice_contains_phrase(&words, THE_NEXT_TIME_PHRASE)
    {
        return crate::cards::builders::InsteadSemantics::FutureReplacement;
    }

    if word_slice_contains_phrase(&words, COUNTERED_THIS_WAY_PHRASE)
        && word_slice_contains_phrase(&words, INSTEAD_OF_PHRASE)
        && word_slice_contains_phrase(&words, GRAVEYARD_WORD_PHRASE)
    {
        return crate::cards::builders::InsteadSemantics::FutureReplacement;
    }

    if word_slice_contains_phrase(&words, INSTEAD_OF_PUTTING_IT_INTO_PHRASE)
        || word_slice_contains_phrase(&words, INSTEAD_OF_PUTTING_THEM_INTO_PHRASE)
    {
        return crate::cards::builders::InsteadSemantics::FutureReplacement;
    }

    crate::cards::builders::InsteadSemantics::SelfReplacement
}

pub(crate) fn find_first_sacrifice_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if is_sacrifice_cost_choice_tag(&choose.tag) {
            return Some(choose.tag.clone());
        }
    }
    None
}

pub(crate) fn find_last_exile_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    let mut found = None;
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if is_exile_cost_choice_tag(&choose.tag) {
            found = Some(choose.tag.clone());
        }
    }
    found
}

pub(crate) fn find_first_unattach_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if is_unattach_cost_choice_tag(&choose.tag) {
            return Some(choose.tag.clone());
        }
    }
    None
}

fn tag_has_prefix(tag: &TagKey, prefix: &str) -> bool {
    tag.as_str().strip_prefix(prefix).is_some()
}

fn is_sacrifice_cost_choice_tag(tag: &TagKey) -> bool {
    tag_has_prefix(tag, SACRIFICE_COST_TAG_PREFIX)
}

fn is_exile_cost_choice_tag(tag: &TagKey) -> bool {
    tag_has_prefix(tag, EXILE_COST_TAG_PREFIX)
}

fn is_unattach_cost_choice_tag(tag: &TagKey) -> bool {
    tag_has_prefix(tag, UNATTACH_COST_TAG_PREFIX)
}

pub(crate) fn value_contains_unbound_x(value: &Value) -> bool {
    match value {
        Value::X | Value::XTimes(_) => true,
        Value::SurfaceHinted { value, .. } => value_contains_unbound_x(value),
        Value::Scaled(value, _) => value_contains_unbound_x(value),
        Value::Add(left, right) => {
            value_contains_unbound_x(left) || value_contains_unbound_x(right)
        }
        _ => false,
    }
}

pub(crate) fn replace_unbound_x_with_value(
    value: Value,
    replacement: &Value,
    clause: &str,
) -> Result<Value, CardTextError> {
    let _ = clause;
    match value {
        Value::X => Ok(replacement.clone()),
        Value::XTimes(multiplier) => {
            if multiplier == 1 {
                return Ok(replacement.clone());
            }
            if let Value::Fixed(fixed) = replacement {
                return Ok(Value::Fixed(fixed * multiplier));
            }
            Ok(Value::Scaled(Box::new(replacement.clone()), multiplier))
        }
        Value::SurfaceHinted { value, hints } => Ok(Value::SurfaceHinted {
            value: Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            hints,
        }),
        Value::Scaled(value, multiplier) => Ok(Value::Scaled(
            Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            multiplier,
        )),
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(replace_unbound_x_with_value(*left, replacement, clause)?),
            Box::new(replace_unbound_x_with_value(*right, replacement, clause)?),
        )),
        other => Ok(other),
    }
}

pub(crate) fn starts_with_activation_cost(tokens: &[OwnedLexToken]) -> bool {
    let Some(first_token) = tokens.first() else {
        return false;
    };
    if mana_pips_from_token(first_token).is_some() {
        return true;
    }
    let Some(word) = first_token.as_word() else {
        return false;
    };
    if matches!(
        word,
        "tap"
            | "t"
            | "pay"
            | "discard"
            | "mill"
            | "sacrifice"
            | "put"
            | "remove"
            | "exile"
            | "return"
            | "e"
    ) {
        return true;
    }
    if str_contains_char(word, '/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    false
}

pub(crate) fn find_activation_cost_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if starts_with_activation_cost(&tokens[idx..]) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn contains_source_from_your_graveyard_phrase(words: &[&str]) -> bool {
    find_window_by(words, 5, |window| {
        SOURCE_FROM_YOUR_GRAVEYARD_PATTERN.matches_words(window)
    })
    .is_some()
}

pub(crate) fn contains_source_from_your_hand_phrase(words: &[&str]) -> bool {
    find_window_by(words, 5, |window| {
        SOURCE_FROM_YOUR_HAND_PATTERN.matches_words(window)
    })
    .is_some()
        || find_window_by(words, 4, |window| {
            SOURCE_FROM_YOUR_HAND_PATTERN.matches_words(window)
        })
        .is_some()
}

pub(crate) fn contains_from_command_zone_phrase(words: &[&str]) -> bool {
    find_window_by(words, 3, |window| {
        FROM_COMMAND_ZONE_PATTERN.matches_words(window)
    })
    .is_some()
}

pub(crate) fn contains_discard_source_phrase(words: &[&str]) -> bool {
    DISCARD_THIS_CARD_PATTERN.matches_words(words)
}

pub(crate) fn is_basic_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

pub(crate) fn join_sentences_with_period(sentences: &[Vec<OwnedLexToken>]) -> Vec<OwnedLexToken> {
    let mut joined = Vec::new();
    for (idx, sentence) in sentences.iter().enumerate() {
        if idx > 0 {
            joined.push(OwnedLexToken::period(TextSpan::synthetic()));
        }
        joined.extend(sentence.clone());
    }
    joined
}

pub(crate) fn split_cost_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_comma()
            || token
                .as_word()
                .is_some_and(|word| AND_WORD_PATTERN.matches_word(word))
        {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool) {
    let has_beginning_of_end_step = BEGINNING_OF_END_STEP_PATTERN.matches_words(tail_words);
    if !has_beginning_of_end_step {
        return (false, false);
    }

    let has_sacrifice_reference = SACRIFICE_DELAY_REFERENCE_PATTERN.matches_words(tail_words);
    let has_exile_reference = EXILE_DELAY_REFERENCE_PATTERN.matches_words(tail_words);

    (has_sacrifice_reference, has_exile_reference)
}

pub(crate) fn token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
    UtilWordView::new(tokens).token_index_for_word_index(word_index)
}

pub(crate) fn remove_first_word(tokens: &[OwnedLexToken], word: &str) -> Vec<OwnedLexToken> {
    let Some(token_idx) = find_token_word(tokens, word) else {
        return tokens.to_vec();
    };
    tokens[..token_idx]
        .iter()
        .chain(tokens[token_idx + 1..].iter())
        .cloned()
        .collect()
}

pub(crate) fn remove_through_first_word(
    tokens: &[OwnedLexToken],
    word: &str,
) -> Vec<OwnedLexToken> {
    let Some(token_idx) = find_token_word(tokens, word) else {
        return Vec::new();
    };
    tokens[token_idx + 1..].to_vec()
}

pub(crate) fn trim_commas(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_comma() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_comma() {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn trim_edge_punctuation_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

pub(crate) fn trim_edge_punctuation(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    trim_edge_punctuation_tokens(tokens).to_vec()
}

pub(crate) fn parser_stacktrace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_STACKTRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!(
        "[parser-flow] stage={stage} clause='{}'",
        crate::runtime_backend::token_word_refs(tokens).join(" ")
    );
}

pub(crate) fn parser_trace_stack(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_stacktrace_enabled() {
        return;
    }
    eprintln!(
        "[parser-trace] stage={stage} clause='{}'",
        crate::runtime_backend::token_word_refs(tokens).join(" ")
    );
    eprintln!("{}", std::backtrace::Backtrace::force_capture());
}

pub(crate) fn starts_with_until_end_of_turn(words: &[&str]) -> bool {
    UNTIL_END_OF_TURN_PREFIX_PATTERN.matches_words(words)
}

pub(crate) fn is_until_end_of_turn(words: &[&str]) -> bool {
    UNTIL_END_OF_TURN_PATTERN.matches_words(words)
}

pub(crate) fn contains_until_end_of_turn(words: &[&str]) -> bool {
    find_window_by(words, 4, is_until_end_of_turn).is_some()
}

pub(crate) fn map_span_to_original(
    span: TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> TextSpan {
    fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
        if byte_idx == 0 {
            return 0;
        }
        let clamped = byte_idx.min(text.len());
        text[..clamped].chars().count()
    }

    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char - 1 >= char_map.len() {
        start_orig
    } else {
        let last_char_idx = end_char - 1;
        let last_orig = char_map[last_char_idx];
        let last_len = original_line[last_orig..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        last_orig + last_len
    };

    TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

pub(crate) fn parse_card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

fn normalized_type_word(word: &str) -> String {
    word.chars()
        .filter_map(|ch| match ch {
            '\'' | '’' | '-' => None,
            _ if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

fn subtype_display_matches_word(subtype: Subtype, candidate: &str) -> bool {
    let base = normalized_type_word(&subtype.to_string());
    if base.is_empty() {
        return false;
    }

    if candidate == base {
        return true;
    }

    if let Some(stem) = str_strip_suffix_char(base.as_str(), 'y')
        && candidate == format!("{stem}ies")
    {
        return true;
    }
    if let Some(stem) = str_strip_suffix(base.as_str(), "fe")
        && candidate == format!("{stem}ves")
    {
        return true;
    }
    if let Some(stem) = str_strip_suffix_char(base.as_str(), 'f')
        && candidate == format!("{stem}ves")
    {
        return true;
    }

    candidate == format!("{base}s")
}

pub(crate) fn parse_supertype_word(word: &str) -> Option<Supertype> {
    match normalized_type_word(word).as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

pub(crate) fn parse_subtype_word(word: &str) -> Option<Subtype> {
    let candidate = normalized_type_word(word);
    if candidate.is_empty() {
        return None;
    }

    match candidate.as_str() {
        "mice" => return Some(Subtype::Mouse),
        "ouphe" => return Some(Subtype::Ouphe),
        "oxen" => return Some(Subtype::Ox),
        "spacecraft" => return Some(Subtype::Spacecraft),
        _ => {}
    }

    for family in [
        SubtypeFamily::Land,
        SubtypeFamily::Creature,
        SubtypeFamily::Artifact,
        SubtypeFamily::Enchantment,
        SubtypeFamily::Spell,
        SubtypeFamily::Planeswalker,
        SubtypeFamily::Battle,
    ] {
        for subtype in family.all_subtypes() {
            if subtype_display_matches_word(*subtype, candidate.as_str()) {
                return Some(*subtype);
            }
        }
    }

    None
}

pub(crate) fn parse_mana_symbol_word_flexible(word: &str) -> Option<ManaSymbol> {
    match word {
        "white" => Some(ManaSymbol::White),
        "blue" => Some(ManaSymbol::Blue),
        "black" => Some(ManaSymbol::Black),
        "red" => Some(ManaSymbol::Red),
        "green" => Some(ManaSymbol::Green),
        "colorless" => Some(ManaSymbol::Colorless),
        _ => None,
    }
}

pub(crate) fn parse_color(word: &str) -> Option<crate::color::ColorSet> {
    crate::color::Color::from_name(word).map(crate::color::ColorSet::from_color)
}

pub(crate) fn parse_non_type(word: &str) -> Option<CardType> {
    let rest = str_strip_prefix(word, "non")?;
    parse_card_type(rest)
}

pub(crate) fn parse_non_supertype(word: &str) -> Option<Supertype> {
    let rest = str_strip_prefix(word, "non")?;
    parse_supertype_word(rest)
}

pub(crate) fn parse_non_color(word: &str) -> Option<crate::color::ColorSet> {
    let rest = str_strip_prefix(word, "non")?;
    parse_color(rest)
}

pub(crate) fn parse_non_subtype(word: &str) -> Option<Subtype> {
    let rest = str_strip_prefix(word, "non")?;
    parse_subtype_flexible(rest)
}

pub(crate) fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
    parse_subtype_word(word)
        .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
        .or_else(|| str_strip_suffix(word, "es").and_then(parse_subtype_word))
        .or_else(|| {
            str_strip_suffix(word, "ves").and_then(|stem| parse_subtype_word(&format!("{stem}f")))
        })
        .or_else(|| {
            str_strip_suffix(word, "ves").and_then(|stem| parse_subtype_word(&format!("{stem}fe")))
        })
}

pub(crate) fn is_source_reference_words(words: &[&str]) -> bool {
    is_this_source_reference_words(words) || source_reference_surface_for_words(words).is_some()
}

fn is_this_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if !THIS_OR_THISS_WORD_PATTERN.matches_word(words[0]) {
        return false;
    }

    if words.len() == 1 {
        return true;
    }

    if words.len() > 2 && OF_WORD_EXACT_PATTERN.matches_word(words[1]) {
        return true;
    }

    if words.len() != 2 {
        return false;
    }

    SOURCE_REFERENCE_NOUN_WORD_PATTERN.matches_word(words[1])
        || parse_card_type(words[1]).is_some()
        || parse_subtype_flexible(words[1]).is_some()
}

pub(crate) fn is_demonstrative_object_head(word: &str) -> bool {
    if matches!(
        word,
        "creature"
            | "creatures"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "spell"
            | "spells"
            | "source"
            | "sources"
            | "token"
            | "tokens"
    ) {
        return true;
    }
    if parse_card_type(word).is_some() {
        return true;
    }
    if let Some(singular) = str_strip_suffix(word, "s") {
        return parse_card_type(singular).is_some();
    }
    false
}

pub(crate) fn is_outlaw_word(word: &str) -> bool {
    OUTLAW_WORD_PATTERN.matches_word(word)
}

pub(crate) fn is_non_outlaw_word(word: &str) -> bool {
    NON_OUTLAW_WORD_PATTERN.matches_word(word)
}

pub(crate) fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
    for subtype in [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ] {
        if !slice_contains(out.as_slice(), &subtype) {
            out.push(subtype);
        }
    }
}

pub(crate) fn is_permanent_type(card_type: CardType) -> bool {
    matches!(
        card_type,
        CardType::Artifact
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
            | CardType::Battle
    )
}

pub(crate) fn parse_zone_word(word: &str) -> Option<Zone> {
    match word {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" | "exiled" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

const ALTERNATIVE_CAST_KIND_PREFIXES: &[(&[&str], AlternativeCastKind)] = &[
    (&["blitz"], AlternativeCastKind::Blitz),
    (&["dash"], AlternativeCastKind::Dash),
    (&["flashback"], AlternativeCastKind::Flashback),
    (&["jump", "start"], AlternativeCastKind::JumpStart),
    (&["jumpstart"], AlternativeCastKind::JumpStart),
    (&["escape"], AlternativeCastKind::Escape),
    (&["madness"], AlternativeCastKind::Madness),
    (&["miracle"], AlternativeCastKind::Miracle),
    (&["suspend"], AlternativeCastKind::Suspend),
];

pub(crate) fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    ALTERNATIVE_CAST_KIND_PREFIXES
        .iter()
        .find_map(|(prefix, kind)| words.starts_with(prefix).then_some((*kind, prefix.len())))
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = str_split_once_char(word, '/')?;
    if str_starts_with_char(power, '+')
        || str_starts_with_char(toughness, '+')
        || str_starts_with_char(power, '-')
        || str_starts_with_char(toughness, '-')
    {
        return None;
    }
    let power = power.parse::<i32>().ok()?;
    let toughness = toughness.parse::<i32>().ok()?;
    Some((power, toughness))
}

pub(crate) fn intern_counter_name(word: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

pub(crate) fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" => Some(CounterType::MinusOneMinusOne),
        "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "+1/+0" => Some(CounterType::PlusOnePlusZero),
        "+0/+1" => Some(CounterType::PlusZeroPlusOne),
        "+1/+2" => Some(CounterType::PlusOnePlusTwo),
        "+2/+2" => Some(CounterType::PlusTwoPlusTwo),
        "-0/-2" => Some(CounterType::MinusZeroMinusTwo),
        "-2/-1" => Some(CounterType::MinusTwoMinusOne),
        "-2/-2" => Some(CounterType::MinusTwoMinusTwo),
        "deathtouch" => Some(CounterType::Deathtouch),
        "decayed" => Some(CounterType::Decayed),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "void" => Some(CounterType::Void),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "blood" => Some(CounterType::Blood),
        "ice" => Some(CounterType::Ice),
        "finality" => Some(CounterType::Finality),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "burden" => Some(CounterType::Named(intern_counter_name("burden"))),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        "luck" => Some(CounterType::Luck),
        "oil" => Some(CounterType::Oil),
        "pressure" => Some(CounterType::Named(intern_counter_name("pressure"))),
        _ => None,
    }
}

pub(crate) fn parse_counter_type_from_tokens(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let token_word_view = UtilWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();

    if let Some(counter_idx) = find_index(token_words.as_slice(), |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    }) {
        if counter_idx == 0 {
            return None;
        }

        let prev = token_words[counter_idx - 1];
        if let Some(counter_type) = parse_counter_type_word(prev) {
            return Some(counter_type);
        }

        if STRIKE_WORD_PATTERN.matches_word(prev) && counter_idx >= 2 {
            match token_words[counter_idx - 2] {
                "double" => return Some(CounterType::DoubleStrike),
                "first" => return Some(CounterType::FirstStrike),
                _ => {}
            }
        }

        if ANOTHER_WORD_PATTERN.matches_word(prev)
            || ironsmith_core::parse_cardinal_word(prev).is_some()
        {
            return None;
        }

        if prev.chars().all(|c| c.is_ascii_alphabetic()) {
            return Some(CounterType::Named(intern_counter_name(prev)));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

fn keyword_action_to_filter_constraint(action: KeywordAction) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};

    if matches!(action, KeywordAction::Decayed) {
        return Some(Marker("decayed"));
    }

    if let KeywordAction::Landwalk(kind) = action {
        let constraint = match kind {
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Island,
                snow: false,
            } => Marker("islandwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Swamp,
                snow: false,
            } => Marker("swampwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Mountain,
                snow: false,
            } => Marker("mountainwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Forest,
                snow: false,
            } => Marker("forestwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Plains,
                snow: false,
            } => Marker("plainswalk"),
            _ => Static(StaticAbilityId::Landwalk),
        };
        return Some(constraint);
    }

    let static_id = keyword_action_to_static_ability(action)?.id();
    match static_id {
        StaticAbilityId::Flying
        | StaticAbilityId::Menace
        | StaticAbilityId::Hexproof
        | StaticAbilityId::Haste
        | StaticAbilityId::FirstStrike
        | StaticAbilityId::DoubleStrike
        | StaticAbilityId::Deathtouch
        | StaticAbilityId::Lifelink
        | StaticAbilityId::Vigilance
        | StaticAbilityId::Trample
        | StaticAbilityId::Reach
        | StaticAbilityId::Defender
        | StaticAbilityId::Flash
        | StaticAbilityId::Phasing
        | StaticAbilityId::Indestructible
        | StaticAbilityId::Shroud
        | StaticAbilityId::Wither
        | StaticAbilityId::Infect
        | StaticAbilityId::Fear
        | StaticAbilityId::Intimidate
        | StaticAbilityId::Shadow
        | StaticAbilityId::Horsemanship
        | StaticAbilityId::Flanking
        | StaticAbilityId::Skulk
        | StaticAbilityId::Changeling => Some(Static(static_id)),
        _ => None,
    }
}

pub(crate) fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    if words.is_empty() {
        return None;
    }
    if MANA_ABILITY_PREFIX_PATTERN.matches_words(words) {
        return Some((FilterKeywordConstraint::Marker("mana ability"), 2));
    }
    if word_is_cycling_keyword_marker(words[0]) {
        return Some((FilterKeywordConstraint::Marker("cycling"), 1));
    }
    if BASIC_LANDCYCLING_PREFIX_PATTERN.matches_words(words) {
        return Some((FilterKeywordConstraint::Marker("cycling"), 2));
    }

    let max_len = words.len().min(4);
    for len in (1..=max_len).rev() {
        let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[..len]);
        let Some(action) = parse_ability_phrase(&tokens) else {
            continue;
        };
        if let Some(constraint) = keyword_action_to_filter_constraint(action) {
            return Some((constraint, len));
        }
    }
    None
}

fn word_has_char_suffix(word: &str, suffix: &[char]) -> bool {
    let mut chars = word.chars().rev();
    suffix
        .iter()
        .rev()
        .all(|expected| chars.next().is_some_and(|ch| ch == *expected))
}

pub(crate) fn word_is_cycling_keyword_marker(word: &str) -> bool {
    cycling_keyword_root(word).is_some()
}

pub(crate) fn cycling_keyword_root(word: &str) -> Option<&str> {
    if CYCLING_WORD_PATTERN.matches_word(word) {
        return Some("");
    }
    if word.chars().count() > CYCLING_SUFFIX_CHARS.len()
        && word_has_char_suffix(word, CYCLING_SUFFIX_CHARS)
    {
        return word.get(..word.len().saturating_sub(CYCLING_SUFFIX_CHARS.len()));
    }
    None
}

pub(crate) fn parse_filter_counter_constraint_words(
    words: &[&str],
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if words.len() < 3 {
        return None;
    }
    let counter_idx = find_index(words, |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    })?;
    if !words
        .get(counter_idx + 1)
        .is_some_and(|word| ON_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !words
        .get(counter_idx + 2)
        .is_some_and(|word| IT_OR_THEM_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let descriptor_words = words[..counter_idx]
        .iter()
        .copied()
        .filter(|word| {
            !OR_MORE_COUNTER_DESCRIPTOR_WORD_PATTERN.matches_word(word)
                && ironsmith_core::parse_cardinal_word(word).is_none()
        })
        .collect::<Vec<_>>();
    let consumed = counter_idx + 3;
    if descriptor_words.is_empty() {
        return Some((crate::filter::CounterConstraint::Any, consumed));
    }
    if descriptor_words
        .first()
        .is_some_and(|word| descriptor_words.len() == 1 && NO_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let descriptor_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&descriptor_words);
    let counter_type = if descriptor_tokens.len() == 1 {
        parse_counter_type_word(descriptor_words[0])
            .unwrap_or_else(|| CounterType::Named(intern_counter_name(descriptor_words[0])))
    } else {
        parse_counter_type_from_tokens(&descriptor_tokens)?
    };
    Some((
        crate::filter::CounterConstraint::Typed(counter_type),
        consumed,
    ))
}

pub(crate) fn apply_filter_keyword_constraint(
    filter: &mut ObjectFilter,
    constraint: FilterKeywordConstraint,
    excluded: bool,
) {
    match constraint {
        FilterKeywordConstraint::Static(ability_id) => {
            if excluded {
                if !slice_contains(filter.excluded_static_abilities.as_slice(), &ability_id) {
                    filter.excluded_static_abilities.push(ability_id);
                }
            } else if !slice_contains(filter.static_abilities.as_slice(), &ability_id) {
                filter.static_abilities.push(ability_id);
            }
        }
        FilterKeywordConstraint::Marker(marker) => {
            if excluded {
                if !filter
                    .excluded_ability_markers
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(marker))
                {
                    filter.excluded_ability_markers.push(marker.to_string());
                }
            } else if !filter
                .ability_markers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(marker))
            {
                filter.ability_markers.push(marker.to_string());
            }
        }
    }
}

pub(crate) fn parse_flashback_keyword_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !words_all
        .first()
        .is_some_and(|word| FLASHBACK_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let (cost, consumed) = leading_mana_symbols_to_oracle(&words_all[1..])?;
    let mut text = format!("Flashback {cost}");
    let tail = &words_all[1 + consumed..];
    if !tail.is_empty() {
        let mut tail_text = tail.join(" ");
        if let Some(first) = tail_text.chars().next() {
            let upper = first.to_ascii_uppercase().to_string();
            let rest = &tail_text[first.len_utf8()..];
            tail_text = format!("{upper}{rest}");
        }
        text.push_str(", ");
        text.push_str(&tail_text);
    }
    Some(vec![KeywordAction::MarkerText(text)])
}

pub(crate) fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    shared_tokens::parse_mana_symbol(part)
}

fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    shared_tokens::parse_mana_symbol_group(raw)
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_tokens::parse_scryfall_mana_cost(raw)
}

pub(crate) fn parse_number_or_x_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?.to_ascii_lowercase();

    if X_WORD_PATTERN.matches_word(word.as_str()) {
        return Some((Value::X, 1));
    }

    let mut words = Vec::new();
    for token in tokens {
        let Some(word) = token.as_word() else {
            break;
        };
        words.push(word);
    }
    let (value, used) = ironsmith_core::parse_cardinal_words(&words)?;
    Some((Value::Fixed(value as i32), used))
}

pub(crate) fn parse_number_or_x_value_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_number_or_x_value(tokens)
}

pub(crate) fn parse_number_word_i32(word: &str) -> Option<i32> {
    if let Ok(value) = word.parse::<i32>() {
        return Some(value);
    }

    ironsmith_core::parse_cardinal_word(word).and_then(|value| value.try_into().ok())
}

pub(crate) fn parse_number_word_u32(word: &str) -> Option<u32> {
    parse_number_word_i32(word).and_then(|value| value.try_into().ok())
}

fn parse_value_expr_term_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.is_empty() {
        return None;
    }

    for (shape, used) in EVENT_AMOUNT_VALUE_PATTERNS {
        if shape.matches_words(words) {
            return Some((Value::EventValue(EventValueSpec::Amount), *used));
        }
    }
    if OTHER_RESULT_VALUE_PATTERN.matches_words(words) {
        return Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::OtherNumber,
            },
            3,
        ));
    }
    if words.len() >= 5 && NUMBER_OF_REMOVED_THIS_WAY_PATTERN.matches_words(words) {
        return Some((Value::EventValue(EventValueSpec::Amount), words.len()));
    }

    if words
        .get(..2)
        .is_some_and(|prefix| matches!(prefix, ["twice", x] if X_WORD_PATTERN.matches_word(x)))
    {
        return Some((Value::XTimes(2), 2));
    }

    if X_WORD_PATTERN.matches_word(words[0]) {
        return Some((Value::X, 1));
    }

    if let Some(value) = parse_number_word_i32(words[0]) {
        return Some((Value::Fixed(value), 1));
    }

    if YOUR_SPEED_VALUE_PATTERN.matches_words(words) {
        return Some((Value::Speed(PlayerFilter::You), 2));
    }
    if TARGET_PLAYER_SPEED_PREFIX_PATTERN.matches_words(words) {
        return Some((Value::Speed(PlayerFilter::target_player()), 3));
    }

    for source_len in (1..words.len()).rev() {
        if let Some(surface) = source_reference_surface_for_possessive_words(&words[..source_len]) {
            match words.get(source_len).copied() {
                Some("power") => {
                    return Some((
                        Value::PowerOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("toughness") => {
                    return Some((
                        Value::ToughnessOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("mana")
                    if words
                        .get(source_len + 1)
                        .is_some_and(|word| VALUE_WORD_PATTERN.matches_word(word)) =>
                {
                    return Some((
                        Value::ManaValueOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 2,
                    ));
                }
                _ => {}
            }
        }
    }

    if SOURCE_POWER_SHORT_PATTERN.matches_words(words) {
        return Some((Value::SourcePower, 2));
    }
    if SOURCE_POWER_LONG_PATTERN.matches_words(words) {
        return Some((Value::SourcePower, 3));
    }
    if SOURCE_TOUGHNESS_SHORT_PATTERN.matches_words(words) {
        return Some((Value::SourceToughness, 2));
    }
    if SOURCE_TOUGHNESS_LONG_PATTERN.matches_words(words) {
        return Some((Value::SourceToughness, 3));
    }
    if SOURCE_MANA_VALUE_SHORT_PATTERN.matches_words(words) {
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Source)), 3));
    }
    if SOURCE_MANA_VALUE_LONG_PATTERN.matches_words(words) {
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Source)), 4));
    }

    let matching_prefix_len = |patterns: &[&[&str]]| {
        patterns
            .iter()
            .find_map(|pattern| words.starts_with(pattern).then_some(pattern.len()))
    };

    if let Some(used) = matching_prefix_len(&[
        &[
            "the", "number", "of", "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
        ],
        &[
            "number", "of", "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
        ],
        &[
            "the", "number", "of", "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ],
        &[
            "number", "of", "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ],
        &[
            "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
        ],
        &[
            "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ],
        &[
            "the", "number", "of", "colors", "of", "mana", "spent", "to", "cast", "it",
        ],
        &[
            "number", "of", "colors", "of", "mana", "spent", "to", "cast", "it",
        ],
        &[
            "the", "number", "of", "colors", "of", "mana", "used", "to", "cast", "it",
        ],
        &[
            "number", "of", "colors", "of", "mana", "used", "to", "cast", "it",
        ],
        &["colors", "of", "mana", "spent", "to", "cast", "it"],
        &["colors", "of", "mana", "used", "to", "cast", "it"],
    ]) {
        return Some((Value::ColorsOfManaSpentToCastThisSpell, used));
    }

    if let Some(used) = matching_prefix_len(&[
        &["that", "creature", "power"],
        &["that", "creatures", "power"],
        &["that", "card", "power"],
        &["that", "cards", "power"],
        &["that", "object", "power"],
        &["that", "objects", "power"],
        &["the", "exiled", "card", "power"],
        &["the", "exiled", "card's", "power"],
        &["the", "exiled", "cards", "power"],
        &["exiled", "card", "power"],
        &["exiled", "card's", "power"],
        &["exiled", "cards", "power"],
        &["the", "exploited", "creature", "power"],
        &["the", "exploited", "creatures", "power"],
        &["exploited", "creature", "power"],
        &["exploited", "creatures", "power"],
        &["the", "sacrificed", "creature", "power"],
        &["the", "sacrificed", "creatures", "power"],
        &["sacrificed", "creature", "power"],
        &["sacrificed", "creatures", "power"],
        &["the", "amassed", "army", "power"],
        &["the", "amassed", "armys", "power"],
        &["amassed", "army", "power"],
        &["amassed", "armys", "power"],
        &["the", "army", "you", "amassed", "power"],
        &["army", "you", "amassed", "power"],
    ]) {
        let tag = if words
            .get(..used)
            .is_some_and(|prefix| EXPLOITED_MARKER_PATTERN.matches_words(prefix))
        {
            crate::tag::EXPLOITED_TAG
        } else {
            IT_TAG
        };
        return Some((
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(tag)))),
            used,
        ));
    }

    if let Some(used) = matching_prefix_len(&[
        &["that", "creature", "toughness"],
        &["that", "creatures", "toughness"],
        &["that", "card", "toughness"],
        &["that", "cards", "toughness"],
        &["that", "object", "toughness"],
        &["that", "objects", "toughness"],
        &["the", "exiled", "card", "toughness"],
        &["the", "exiled", "card's", "toughness"],
        &["the", "exiled", "cards", "toughness"],
        &["exiled", "card", "toughness"],
        &["exiled", "card's", "toughness"],
        &["exiled", "cards", "toughness"],
        &["the", "exploited", "creature", "toughness"],
        &["the", "exploited", "creatures", "toughness"],
        &["exploited", "creature", "toughness"],
        &["exploited", "creatures", "toughness"],
        &["the", "sacrificed", "creature", "toughness"],
        &["the", "sacrificed", "creatures", "toughness"],
        &["sacrificed", "creature", "toughness"],
        &["sacrificed", "creatures", "toughness"],
        &["the", "amassed", "army", "toughness"],
        &["the", "amassed", "armys", "toughness"],
        &["amassed", "army", "toughness"],
        &["amassed", "armys", "toughness"],
        &["the", "army", "you", "amassed", "toughness"],
        &["army", "you", "amassed", "toughness"],
    ]) {
        let tag = if words
            .get(..used)
            .is_some_and(|prefix| EXPLOITED_MARKER_PATTERN.matches_words(prefix))
        {
            crate::tag::EXPLOITED_TAG
        } else {
            IT_TAG
        };
        return Some((
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(tag)))),
            used,
        ));
    }

    if let Some(used) = matching_prefix_len(&[
        &["that", "spell", "mana", "value"],
        &["that", "spell's", "mana", "value"],
        &["that", "spells", "mana", "value"],
        &["that", "permanent", "mana", "value"],
        &["that", "permanent's", "mana", "value"],
        &["that", "permanents", "mana", "value"],
        &["that", "equipment", "mana", "value"],
        &["that", "equipment's", "mana", "value"],
        &["that", "equipments", "mana", "value"],
        &["that", "object", "mana", "value"],
        &["that", "object's", "mana", "value"],
        &["that", "objects", "mana", "value"],
    ]) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
            used,
        ));
    }

    if let Some(used) = matching_prefix_len(&[
        &["the", "card", "mana", "value"],
        &["the", "card's", "mana", "value"],
        &["the", "cards", "mana", "value"],
        &["that", "card", "mana", "value"],
        &["that", "card's", "mana", "value"],
        &["that", "cards", "mana", "value"],
        &["the", "revealed", "card", "mana", "value"],
        &["the", "revealed", "cards", "mana", "value"],
        &["revealed", "card", "mana", "value"],
        &["revealed", "cards", "mana", "value"],
        &["the", "mana", "value", "of", "the", "revealed", "card"],
        &["the", "mana", "value", "of", "the", "revealed", "cards"],
        &["mana", "value", "of", "the", "revealed", "card"],
        &["mana", "value", "of", "the", "revealed", "cards"],
        &["the", "exiled", "card", "mana", "value"],
        &["the", "exiled", "cards", "mana", "value"],
        &["exiled", "card", "mana", "value"],
        &["exiled", "cards", "mana", "value"],
        &["the", "mana", "value", "of", "the", "exiled", "card"],
        &["the", "mana", "value", "of", "the", "exiled", "cards"],
        &["mana", "value", "of", "the", "exiled", "card"],
        &["mana", "value", "of", "the", "exiled", "cards"],
        &["the", "exiled", "spell", "mana", "value"],
        &["the", "exiled", "spells", "mana", "value"],
        &["exiled", "spell", "mana", "value"],
        &["exiled", "spells", "mana", "value"],
        &["the", "sacrificed", "creature", "mana", "value"],
        &["the", "sacrificed", "creatures", "mana", "value"],
        &["sacrificed", "creature", "mana", "value"],
        &["sacrificed", "creatures", "mana", "value"],
        &["the", "amassed", "army", "mana", "value"],
        &["the", "amassed", "armys", "mana", "value"],
        &["amassed", "army", "mana", "value"],
        &["amassed", "armys", "mana", "value"],
        &["the", "mana", "value", "of", "the", "amassed", "army"],
        &["the", "mana", "value", "of", "the", "amassed", "armys"],
        &["mana", "value", "of", "the", "amassed", "army"],
        &["mana", "value", "of", "the", "amassed", "armys"],
        &[
            "the", "mana", "value", "of", "the", "army", "you", "amassed",
        ],
        &["mana", "value", "of", "the", "army", "you", "amassed"],
    ]) {
        let tag = if matches!(
            words.get(..used),
            Some(["the", "exiled", "card", "mana", "value"])
                | Some(["the", "exiled", "cards", "mana", "value"])
                | Some(["exiled", "card", "mana", "value"])
                | Some(["exiled", "cards", "mana", "value"])
                | Some(["the", "mana", "value", "of", "the", "exiled", "card"])
                | Some(["the", "mana", "value", "of", "the", "exiled", "cards"])
                | Some(["mana", "value", "of", "the", "exiled", "card"])
                | Some(["mana", "value", "of", "the", "exiled", "cards"])
                | Some(["the", "exiled", "spell", "mana", "value"])
                | Some(["the", "exiled", "spells", "mana", "value"])
                | Some(["exiled", "spell", "mana", "value"])
                | Some(["exiled", "spells", "mana", "value"])
        ) {
            TagKey::from(crate::tag::SOURCE_EXILED_TAG)
        } else if matches!(
            words.get(..used),
            Some(["the", "revealed", "card", "mana", "value"])
                | Some(["the", "revealed", "cards", "mana", "value"])
                | Some(["revealed", "card", "mana", "value"])
                | Some(["revealed", "cards", "mana", "value"])
                | Some(["the", "mana", "value", "of", "the", "revealed", "card"])
                | Some(["the", "mana", "value", "of", "the", "revealed", "cards"])
                | Some(["mana", "value", "of", "the", "revealed", "card"])
                | Some(["mana", "value", "of", "the", "revealed", "cards"])
        ) {
            TagKey::from("__public_revealed")
        } else {
            TagKey::from(IT_TAG)
        };
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Tagged(tag))), used));
    }

    let mut idx = 0usize;
    if THE_WORD_PATTERN.matches_word(words[idx]) {
        idx += 1;
    }
    if !words
        .get(idx..idx + 2)
        .is_some_and(|words| NUMBER_OF_PATTERN.matches_words(words))
    {
        return None;
    }
    idx += 2;

    let mut counter_idx = idx;
    if words
        .get(counter_idx)
        .is_some_and(|word| is_article(word) || ONE_WORD_PATTERN.matches_word(word))
    {
        counter_idx += 1;
    }

    let mut parsed_counter_type = None;
    if let Some(word) = words.get(counter_idx).copied()
        && let Some(counter_type) = parse_counter_type_word(word)
    {
        parsed_counter_type = Some(counter_type);
        counter_idx += 1;
    }

    if words
        .get(counter_idx)
        .is_some_and(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
        && words
            .get(counter_idx + 1)
            .is_some_and(|word| ON_WORD_PATTERN.matches_word(word))
    {
        let reference_start = counter_idx + 2;
        let mut reference_end = reference_start;
        while reference_end < words.len()
            && !PLUS_OR_MINUS_WORD_PATTERN.matches_word(words[reference_end])
        {
            reference_end += 1;
        }
        let reference = &words[reference_start..reference_end];
        if SOURCE_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
            let value = match parsed_counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            };
            return Some((value, reference_end));
        }
        if TAGGED_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
            let value = Value::CountersOn(
                Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::cards::builders::IT_TAG,
                ))),
                parsed_counter_type,
            );
            return Some((value, reference_end));
        }
    }

    let filter_start = idx;
    let mut filter_end = filter_start;
    while filter_end < words.len() && !PLUS_OR_MINUS_WORD_PATTERN.matches_word(words[filter_end]) {
        filter_end += 1;
    }
    if filter_end <= filter_start {
        return None;
    }
    let filter_words = &words[filter_start..filter_end];
    if BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(filter_words) {
        let scope_start = filter_start + 4;
        let filter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[scope_start..filter_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        return Some((Value::BasicLandTypesAmong(filter), filter_end));
    }
    if COLORS_AMONG_PREFIX_PATTERN.matches_words(filter_words) {
        let mut scope_start = filter_start + 2;
        if words
            .get(scope_start)
            .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
        {
            scope_start += 1;
        }
        let filter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[scope_start..filter_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        return Some((Value::ColorsAmong(filter), filter_end));
    }
    if DIFFERENT_POWERS_AMONG_PREFIX_PATTERN.matches_words(filter_words) {
        let scope_start = if filter_words
            .get(2)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        {
            filter_start + 3
        } else {
            filter_start + 4
        };
        let filter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[scope_start..filter_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        return Some((Value::DistinctPowers(filter), filter_end));
    }
    if SPELL_CAST_THIS_TURN_COUNT_PATTERN.matches_words(filter_words) {
        for (suffix_pattern, suffix_len, player) in SPELL_CAST_THIS_TURN_SUFFIX_PATTERNS {
            if !suffix_pattern.matches_words(filter_words) {
                continue;
            }
            let count_filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &filter_words[..filter_words.len().saturating_sub(*suffix_len)],
            );
            if let Ok(filter) = parse_object_filter(&count_filter_tokens, false) {
                let exclude_source = count_filter_tokens.iter().any(|token| {
                    token
                        .as_word()
                        .is_some_and(|word| OTHER_WORD_PATTERN.matches_word(word))
                });
                return Some((
                    Value::SpellsCastThisTurnMatching {
                        player: player.clone(),
                        filter,
                        exclude_source,
                    },
                    filter_end,
                ));
            }
        }
    }
    let filter_tokens =
        crate::runtime_backend::lexer::synthetic_word_tokens(&words[filter_start..filter_end]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some((Value::Count(filter), filter_end))
}

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    let (mut value, mut used) = parse_value_expr_term_words(words)?;

    while used < words.len() {
        let operator = words[used];
        if !PLUS_OR_MINUS_WORD_PATTERN.matches_word(operator) {
            break;
        }

        let (rhs, rhs_used) = parse_value_expr_term_words(&words[used + 1..])?;
        used += 1 + rhs_used;

        let rhs = if MINUS_WORD_PATTERN.matches_word(operator) {
            match rhs {
                Value::Fixed(fixed) => Value::Fixed(-fixed),
                _ => return None,
            }
        } else {
            rhs
        };

        value = Value::Add(Box::new(value), Box::new(rhs));
    }

    Some((value, used))
}

pub(crate) fn parse_value_expr(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let word_view = UtilWordView::new(tokens);
    let words = word_view.to_word_refs();
    let (value, used_words) = parse_value_expr_words(&words)?;
    let used = token_index_for_word_index(tokens, used_words).unwrap_or(tokens.len());
    Some((value, used))
}

pub(crate) fn parse_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_value_expr(tokens)
}

fn is_that_player_or_that_objects_controller_phrase(words: &[&str]) -> bool {
    words.len() >= 6
        && THAT_PLAYER_OR_THAT_PREFIX_PATTERN.matches_words(words)
        && CONTROLLED_OBJECT_PLURAL_WORD_PATTERN.matches_word(words[4])
        && CONTROLLER_WORD_PATTERN.matches_word(words[5])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectAst {
    Player(PlayerAst),
    This,
}

pub(crate) fn parse_subject(tokens: &[OwnedLexToken]) -> SubjectAst {
    let word_view = UtilWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return SubjectAst::This;
    }

    let mut start = 0usize;
    if ANY_NUMBER_PREFIX_PATTERN.matches_words(&words) {
        start = if words
            .get(2)
            .is_some_and(|word| OF_WORD_PATTERN.matches_word(word))
        {
            3
        } else {
            2
        };
    }

    let mut slice = &words[start..];
    while slice
        .first()
        .is_some_and(|word| THEN_OR_AND_WORD_PATTERN.matches_word(word))
    {
        slice = &slice[1..];
    }
    while slice
        .first()
        .is_some_and(|word| EACH_WORD_PATTERN.matches_word(word))
    {
        slice = &slice[1..];
    }
    if slice
        .first()
        .is_some_and(|word| parse_number_word_u32(word).is_some() || word.parse::<u32>().is_ok())
    {
        slice = &slice[1..];
    }

    if MOST_CARDS_IN_HAND_SUBJECT_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::MostCardsInHand);
    }
    if MOST_LIFE_SUBJECT_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::MostLifeTied);
    }
    if LOWEST_LIFE_SUBJECT_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::LowestLifeTied);
    }

    if let Some(have_idx) = find_index(slice, |word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word)) {
        if have_idx + 1 < slice.len() {
            slice = &slice[have_idx + 1..];
        }
    }

    if YOU_OR_YOUR_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if TARGET_OPPONENT_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }

    if TARGET_PLAYER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::Target);
    }
    if PLAYER_OF_YOUR_CHOICE_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::Chosen);
    }

    if OPPONENT_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::Opponent);
    }
    if OTHER_PLAYER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::NotYou);
    }

    if DEFENDING_PLAYER_EDGE_PATTERN.matches_words(slice)
        || DEFENDING_PLAYER_SUFFIX_PATTERN.matches_words(slice)
    {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if ATTACKING_PLAYER_EDGE_PATTERN.matches_words(slice)
        || ATTACKING_PLAYER_SUFFIX_PATTERN.matches_words(slice)
    {
        return SubjectAst::Player(PlayerAst::Attacking);
    }

    if THAT_PLAYER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if VOTER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if CHOSEN_PLAYER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::Chosen);
    }

    if is_that_player_or_that_objects_controller_phrase(slice) {
        return SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController);
    }

    if THAT_PLAYERS_OR_THEIR_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if OWNERS_OF_THOSE_OBJECTS_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice.len() >= 3
        && THAT_WORD_PATTERN.matches_word(slice[0])
        && CONTROLLER_OR_OWNER_WORD_PATTERN.matches_word(slice[2])
        && CONTROLLED_OBJECT_PLURAL_WORD_PATTERN.matches_word(slice[1])
    {
        let player = if OWNER_WORD_PATTERN.matches_word(slice[2]) {
            PlayerAst::ItsOwner
        } else {
            PlayerAst::ItsController
        };
        return SubjectAst::Player(player);
    }

    if ITS_CONTROLLER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if ITS_OR_THEIR_OWNER_PREFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if THIS_PREFIX_PATTERN.matches_words(slice)
        && slice
            .last()
            .is_some_and(|word| CONTROLLER_WORD_PATTERN.matches_word(word))
    {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if THIS_PREFIX_PATTERN.matches_words(slice)
        && slice
            .last()
            .is_some_and(|word| OWNER_WORD_PATTERN.matches_word(word))
    {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if ITS_OR_THEIR_CONTROLLER_SUFFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if ITS_OR_THEIR_OWNER_SUFFIX_PATTERN.matches_words(slice) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice
        .first()
        .is_some_and(|word| THIS_OR_THISS_WORD_PATTERN.matches_word(word))
    {
        return SubjectAst::This;
    }

    SubjectAst::This
}

pub(crate) fn span_from_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    token_slice_span(tokens)
}

pub(crate) fn parse_number(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let token = tokens.first()?;
    let word = token
        .as_word()
        .unwrap_or_else(|| token.parser_text())
        .to_ascii_lowercase();

    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }

    match word.as_str() {
        "once" => return Some((1, 1)),
        "twice" => return Some((2, 1)),
        _ => {}
    }

    let trimmed_trailing_punctuation = word.trim_end_matches(|ch: char| !ch.is_ascii_digit());
    if trimmed_trailing_punctuation.len() < word.len()
        && !trimmed_trailing_punctuation.is_empty()
        && trimmed_trailing_punctuation
            .chars()
            .all(|ch| ch.is_ascii_digit())
    {
        if let Ok(value) = trimmed_trailing_punctuation.parse::<u32>() {
            return Some((value, 1));
        }
    }

    let mut words = Vec::new();
    for token in tokens {
        let word = token.as_word().unwrap_or_else(|| token.parser_text());
        if word.is_empty() {
            break;
        }
        words.push(word);
    }
    let (value, used) = ironsmith_core::parse_cardinal_words(&words)?;
    Some((value, used))
}

pub(crate) fn parse_quantity_comparison_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing quantity in {error_context}"
        )));
    }

    if EXACTLY_WORD_PATTERN.matches_token(&tokens[0]) {
        let (value, used) = parse_number(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((crate::effect::Comparison::Equal(value as i32), used + 1));
    }

    if NO_WORD_PATTERN.matches_token(&tokens[0])
        && token_slice_at_is_any(tokens, 1, &["more", "greater"])
        && token_slice_at_is(tokens, 2, "than")
    {
        let (value, used) = parse_number(tokens.get(3..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((
            crate::effect::Comparison::LessThanOrEqual(value as i32),
            used + 3,
        ));
    }

    if NO_WORD_PATTERN.matches_token(&tokens[0]) {
        return Ok((crate::effect::Comparison::LessThanOrEqual(0), 1));
    }

    if AT_LEAST_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens)) {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((
            crate::effect::Comparison::GreaterThanOrEqual(value as i32),
            used + 2,
        ));
    }

    if AT_MOST_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens)) {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((
            crate::effect::Comparison::LessThanOrEqual(value as i32),
            used + 2,
        ));
    }

    if FEWER_OR_LESS_WORD_PATTERN.matches_token(&tokens[0])
        && tokens
            .get(1)
            .is_some_and(|token| THAN_WORD_PATTERN.matches_token(token))
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((crate::effect::Comparison::LessThan(value as i32), used + 2));
    }

    if MORE_OR_GREATER_WORD_PATTERN.matches_token(&tokens[0])
        && tokens
            .get(1)
            .is_some_and(|token| THAN_WORD_PATTERN.matches_token(token))
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!("missing quantity in {error_context}"))
        })?;
        return Ok((
            crate::effect::Comparison::GreaterThan(value as i32),
            used + 2,
        ));
    }

    if let Some((value, used)) = parse_number(tokens) {
        let value = value as i32;
        let first_word = tokens.first().and_then(OwnedLexToken::as_word);
        if article_implies_min_one
            && first_word.is_some_and(|word| QUANTITY_ARTICLE_WORD_PATTERN.matches_word(word))
        {
            return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), used));
        }
        if token_slice_at_is(tokens, used, "or")
            && token_slice_at_is_any(tokens, used + 1, &["more", "greater"])
        {
            return Ok((
                crate::effect::Comparison::GreaterThanOrEqual(value),
                used + 2,
            ));
        }
        if token_slice_at_is(tokens, used, "or")
            && token_slice_at_is_any(tokens, used + 1, &["less", "fewer"])
        {
            return Ok((crate::effect::Comparison::LessThanOrEqual(value), used + 2));
        }
        return Ok((crate::effect::Comparison::Equal(value), used));
    }

    if allow_default_one {
        return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), 0));
    }

    Err(CardTextError::ParseError(format!(
        "missing quantity in {error_context}"
    )))
}

pub(crate) fn comparison_to_at_least_threshold(
    comparison: &crate::effect::Comparison,
) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        crate::effect::Comparison::Equal(value) if *value >= 0 => Some(*value as u32),
        _ => None,
    }
}

pub(crate) fn comparison_to_strict_at_least_threshold(
    comparison: &crate::effect::Comparison,
) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        _ => None,
    }
}

pub(crate) fn comparison_to_strict_at_most_threshold(
    comparison: &crate::effect::Comparison,
) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::LessThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::LessThan(value) if *value > 0 => Some((*value - 1) as u32),
        _ => None,
    }
}

pub(crate) fn comparison_to_value_comparison_operator(
    comparison: crate::effect::Comparison,
) -> Option<(crate::effect::ValueComparisonOperator, i32)> {
    match comparison {
        crate::effect::Comparison::GreaterThan(value) => {
            Some((crate::effect::ValueComparisonOperator::GreaterThan, value))
        }
        crate::effect::Comparison::GreaterThanOrEqual(value) => Some((
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            value,
        )),
        crate::effect::Comparison::Equal(value) => {
            Some((crate::effect::ValueComparisonOperator::Equal, value))
        }
        crate::effect::Comparison::LessThan(value) => {
            Some((crate::effect::ValueComparisonOperator::LessThan, value))
        }
        crate::effect::Comparison::LessThanOrEqual(value) => Some((
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            value,
        )),
        crate::effect::Comparison::NotEqual(value) => {
            Some((crate::effect::ValueComparisonOperator::NotEqual, value))
        }
        crate::effect::Comparison::BetweenInclusive(_, _) | crate::effect::Comparison::OneOf(_) => {
            None
        }
    }
}

pub(crate) fn parse_greater_than_or_equal_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<Option<(u32, usize)>, CardTextError> {
    let (comparison, used) = parse_quantity_comparison_prefix(
        tokens,
        allow_default_one,
        article_implies_min_one,
        error_context,
    )?;
    Ok(comparison_to_strict_at_least_threshold(&comparison).map(|count| (count, used)))
}

pub(crate) fn parse_less_than_or_equal_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<Option<(u32, usize)>, CardTextError> {
    let (comparison, used) = parse_quantity_comparison_prefix(
        tokens,
        allow_default_one,
        article_implies_min_one,
        error_context,
    )?;
    Ok(comparison_to_strict_at_most_threshold(&comparison).map(|count| (count, used)))
}

pub(crate) fn parse_target_count_range_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let (first, first_used) = parse_number(tokens)?;
    let or_idx = first_used;
    if !token_slice_at_is(tokens, or_idx, "or") {
        return None;
    }
    let (second, second_used) = parse_number(&tokens[or_idx + 1..])?;
    if second < first {
        return None;
    }
    Some((
        ChoiceCount {
            min: first as usize,
            max: Some(second as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        },
        first_used + 1 + second_used,
    ))
}

pub(crate) fn parse_choice_count_token_prefix(
    tokens: &[OwnedLexToken],
) -> (ChoiceCount, Vec<OwnedLexToken>) {
    if let Some((count, used)) = parse_choice_count_token_prefix_consumed(tokens) {
        return (count, trim_commas(&tokens[used..]));
    }
    (ChoiceCount::exactly(1), tokens.to_vec())
}

pub(crate) fn parse_choice_count_token_prefix_consumed(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let words = token_word_refs(tokens);
    if ANY_NUMBER_PREFIX_PATTERN.matches_words(&words) {
        let used = if token_slice_at_is(tokens, 2, "of") {
            3
        } else {
            2
        };
        return Some((ChoiceCount::any_number(), used));
    }
    if UP_TO_PREFIX_PATTERN.matches_words(&words) {
        if tokens
            .get(2)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| X_WORD_PATTERN.matches_word(word))
        {
            return Some((ChoiceCount::up_to_dynamic_x(), 3));
        }
        if let Some((value, used)) = parse_number(&tokens[2..]) {
            return Some((
                ChoiceCount {
                    min: 0,
                    max: Some(value as usize),
                    dynamic_x: false,
                    up_to_x: false,
                    random: false,
                },
                2 + used,
            ));
        }
    }
    if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| X_WORD_PATTERN.matches_word(word))
    {
        return Some((ChoiceCount::dynamic_x(), 1));
    }
    if let Some((value, used)) = parse_number(tokens) {
        return Some((ChoiceCount::exactly(value as usize), used));
    }
    None
}

pub(crate) fn parse_choice_or_range_count_token_prefix_consumed(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    parse_target_count_range_prefix(tokens)
        .or_else(|| parse_choice_count_token_prefix_consumed(tokens))
}

pub(crate) fn parse_choice_count_before_target_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let (count, used) = parse_choice_or_range_count_token_prefix_consumed(tokens)?;
    tokens
        .get(used)
        .is_some_and(|token| TARGET_OR_TARGETS_WORD_PATTERN.matches_token(token))
        .then_some((count, used))
}

pub(crate) fn parse_choice_count_word_prefix(words: &[&str]) -> Option<(ChoiceCount, usize)> {
    if ANY_NUMBER_PREFIX_PATTERN.matches_words(words) {
        let used = if words
            .get(2)
            .is_some_and(|word| OF_WORD_PATTERN.matches_word(word))
        {
            3
        } else {
            2
        };
        return Some((ChoiceCount::any_number(), used));
    }
    if UP_TO_PREFIX_PATTERN.matches_words(words) {
        if words
            .get(2)
            .is_some_and(|word| X_WORD_PATTERN.matches_word(word))
        {
            return Some((ChoiceCount::up_to_dynamic_x(), 3));
        }
        let count_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
            words.get(2..).unwrap_or_default(),
        );
        if let Some((value, used)) = parse_number(&count_tokens) {
            return Some((
                ChoiceCount {
                    min: 0,
                    max: Some(value as usize),
                    dynamic_x: false,
                    up_to_x: false,
                    random: false,
                },
                2 + used,
            ));
        }
    }
    if words
        .first()
        .is_some_and(|word| X_WORD_PATTERN.matches_word(word))
    {
        return Some((ChoiceCount::dynamic_x(), 1));
    }
    let count_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_number(&count_tokens).map(|(value, used)| (ChoiceCount::exactly(value as usize), used))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parse_subject_recognizes_a_player_of_your_choice() {
        let tokens = lex_line("A player of your choice adds {C}.", 0).unwrap();
        assert_eq!(
            parse_subject(&tokens),
            SubjectAst::Player(PlayerAst::Chosen)
        );
    }

    #[test]
    fn parse_subject_recognizes_they_as_that_player() {
        let tokens = lex_line("they lose 5 life", 0).unwrap();
        assert_eq!(parse_subject(&tokens), SubjectAst::Player(PlayerAst::That));
    }

    #[test]
    fn parse_subject_recognizes_each_other_player() {
        let tokens = lex_line("each other player", 0).unwrap();
        assert_eq!(
            parse_subject(&tokens),
            SubjectAst::Player(PlayerAst::NotYou)
        );
    }

    #[test]
    fn parse_target_phrase_recognizes_source_name_with_internal_article() {
        let tokens = lex_line("Kraven the Hunter", 0).unwrap();
        let target = with_source_reference_context("Kraven the Hunter", || {
            parse_target_phrase(&tokens).expect("source name with internal article should parse")
        });

        assert!(
            matches!(target, TargetAst::Source(_))
                || matches!(target, TargetAst::Object(ref filter, _, _) if filter.source),
            "expected source target, got {target:?}"
        );
    }

    #[test]
    fn parse_power_toughness_accepts_star_plus_forms() {
        assert_eq!(
            parse_power_toughness("*+1/2"),
            Some(PowerToughness::new(PtValue::StarPlus(1), PtValue::Fixed(2)))
        );
        assert_eq!(
            parse_power_toughness("1+*/0.5"),
            Some(PowerToughness::new(PtValue::StarPlus(1), PtValue::Fixed(0)))
        );
        assert_eq!(
            parse_power_toughness("*/*"),
            Some(PowerToughness::new(PtValue::Star, PtValue::Star))
        );
    }

    #[test]
    fn parse_unsigned_pt_word_rejects_signed_components() {
        assert_eq!(parse_unsigned_pt_word("2/3"), Some((2, 3)));
        assert_eq!(parse_unsigned_pt_word("+2/3"), None);
        assert_eq!(parse_unsigned_pt_word("2/-3"), None);
    }

    #[test]
    fn parse_target_phrase_recognizes_bare_the_other_reference() {
        let tokens = lex_line("the other", 0).unwrap();
        let target = parse_target_phrase(&tokens).expect("the other should parse as other target");

        assert!(
            matches!(target, TargetAst::AnyOtherTarget(_)),
            "expected AnyOtherTarget, got {target:?}"
        );
    }

    #[test]
    fn parse_number_accepts_numeric_word_with_trailing_period() {
        let tokens = lex_line("2.", 0).unwrap();
        let (value, used) =
            parse_number(&tokens).expect("number with trailing period should parse");
        assert_eq!(value, 2);
        assert_eq!(used, 1);
    }

    #[test]
    fn parse_for_each_count_value_words_binds_revealed_this_way_to_it_tag() {
        let words = ["for", "each", "card", "revealed", "this", "way"];

        let (value, used_words) =
            parse_for_each_count_value_words(&words).expect("count phrase should parse");

        assert_eq!(used_words, words.len());
        let Value::Count(filter) = value else {
            panic!("expected a counted object filter, got {value:?}");
        };
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }),
            "expected the revealed-this-way count to bind to IT_TAG, got {filter:?}"
        );
    }

    #[test]
    fn parse_for_each_count_value_words_handles_cards_youve_drawn_this_turn() {
        let words = ["for", "each", "card", "you've", "drawn", "this", "turn"];

        let (value, used_words) =
            parse_for_each_count_value_words(&words).expect("count phrase should parse");

        assert_eq!(used_words, words.len());
        assert_eq!(value, Value::MaxCardsDrawnThisTurn(PlayerFilter::You));
    }
}

pub(crate) fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

fn choice_count_from_value(value: &Value, up_to: bool) -> ChoiceCount {
    match value {
        Value::X => {
            if up_to {
                ChoiceCount::up_to_dynamic_x()
            } else {
                ChoiceCount::dynamic_x()
            }
        }
        Value::Fixed(count) => {
            let count = (*count).max(0) as usize;
            if up_to {
                ChoiceCount::up_to(count)
            } else {
                ChoiceCount::exactly(count)
            }
        }
        other => unreachable!("unsupported target-count value {other:?}"),
    }
}

pub(crate) fn is_source_from_your_graveyard_words(words: &[&str]) -> bool {
    words.len() >= 4 && SOURCE_FROM_YOUR_GRAVEYARD_MARKER_PATTERN.matches_words(words)
}

pub(crate) fn parse_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let all_words = crate::runtime_backend::token_word_refs(tokens);
    let count_prefix_tokens = if all_words.len() >= 2
        && EACH_WORD_PATTERN.matches_word(all_words[0])
        && OF_WORD_PATTERN.matches_word(all_words[1])
    {
        tokens.get(2..).unwrap_or_default()
    } else {
        tokens
    };
    if let Some((count, used)) = parse_choice_count_before_target_prefix(count_prefix_tokens) {
        if count_prefix_tokens.len() == used + 1 {
            return Ok(TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(tokens))),
                count,
            ));
        }
    }

    match parse_target_phrase_inner(tokens) {
        Ok(target) => Ok(target),
        Err(err) => {
            if let Some(except_idx) = all_words
                .iter()
                .position(|word| EXCEPT_WORD_PATTERN.matches_word(word))
                && except_idx > 0
                && let Some(token_end) = token_index_for_word_index(tokens, except_idx)
            {
                let candidate = trim_commas(&tokens[..token_end]);
                if !candidate.is_empty()
                    && let Ok(target) = parse_target_phrase_inner(&candidate)
                {
                    return Ok(target);
                }
                if all_words
                    .first()
                    .is_some_and(|word| COPY_WORD_PATTERN.matches_word(word))
                    && let Some(token_start) = token_index_for_word_index(tokens, 1)
                {
                    let candidate = trim_commas(&tokens[token_start..token_end]);
                    if !candidate.is_empty()
                        && let Ok(target) = parse_target_phrase_inner(&candidate)
                    {
                        return Ok(target);
                    }
                }
            }
            if all_words
                .first()
                .is_some_and(|word| PARSE_TARGET_LEADING_CONDITION_WORD_PATTERN.matches_word(word))
            {
                for word_start in (1..all_words.len()).rev() {
                    let Some(token_start) = token_index_for_word_index(tokens, word_start) else {
                        continue;
                    };
                    let candidate = trim_commas(&tokens[token_start..]);
                    let candidate_words = crate::runtime_backend::token_word_refs(&candidate);
                    if candidate_words.is_empty() {
                        continue;
                    }
                    if candidate_words.first().is_some_and(|word| {
                        PARSE_TARGET_SPLIT_PREFIX_WORD_PATTERN.matches_word(word)
                    }) {
                        continue;
                    }
                    if let Ok(target) = parse_target_phrase_inner(&candidate) {
                        return Ok(target);
                    }
                }
            }
            Err(err)
        }
    }
}

fn tagged_it_owner_or_controller_player_filter(word: &str) -> PlayerFilter {
    if matches!(word, "owner" | "owners") {
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
    } else {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
    }
}

fn parse_target_phrase_inner(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    let stripped_random_tokens;
    while token_slice_first_is(tokens, "then") {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    let mut random_choice = false;
    let token_word_view = UtilWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();
    if YOUR_OPPONENTS_TARGET_PATTERN.matches_words(token_words.as_slice()) {
        return Ok(TargetAst::Player(
            PlayerFilter::Opponent,
            span_from_tokens(tokens),
        ));
    }
    if DEFENDING_PLAYER_CHOICE_TARGET_PATTERN.matches_words(token_words.as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported defending player's choice target phrase '{}'",
            token_words.join(" ")
        )));
    }
    if CHOSEN_AT_RANDOM_TAIL_PATTERN.matches_words(token_words.as_slice()) {
        if let Some(random_idx) = token_word_view.token_index_for_word_index(token_words.len() - 3)
        {
            tokens = &tokens[..random_idx];
            random_choice = true;
        }
    } else if AT_RANDOM_TAIL_PATTERN.matches_words(token_words.as_slice())
        && let Some(random_idx) = token_word_view.token_index_for_word_index(token_words.len() - 2)
    {
        tokens = &tokens[..random_idx];
        random_choice = true;
    } else if let Some(random_word_idx) = token_words
        .windows(2)
        .position(|window| window == ["at", "random"])
        && let Some(random_start) = token_word_view.token_index_for_word_index(random_word_idx)
    {
        let random_end = token_word_view
            .token_index_for_word_index(random_word_idx + 2)
            .unwrap_or(tokens.len());
        stripped_random_tokens = tokens
            .iter()
            .take(random_start)
            .chain(tokens.iter().skip(random_end))
            .cloned()
            .collect::<Vec<_>>();
        tokens = &stripped_random_tokens;
        random_choice = true;
    }

    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);
    let mut target_count: Option<ChoiceCount> = None;
    let mut explicit_target = false;

    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if ANY_TARGET_PATTERN.matches_words(&all_words) {
        return Ok(TargetAst::AnyTarget(span));
    }
    if ANY_OTHER_TARGET_PATTERN.matches_words(&all_words) {
        return Ok(TargetAst::AnyOtherTarget(span));
    }
    if UP_TO_PREFIX_PATTERN.matches_words(all_words.as_slice())
        && all_words
            .last()
            .is_some_and(|word| TARGET_OR_TARGETS_WORD_PATTERN.matches_word(word))
        && let Some((value, _)) = parse_number_or_x_value(&tokens[2..])
    {
        let target_words = crate::runtime_backend::token_word_refs(&tokens[3..]);
        let target = if OTHER_TARGET_PATTERN.matches_words(&target_words) {
            TargetAst::AnyOtherTarget(span)
        } else {
            TargetAst::AnyTarget(span)
        };
        return Ok(TargetAst::WithCount(
            Box::new(target),
            choice_count_from_value(&value, true),
        ));
    }
    if all_words.len() >= 4
        && all_words
            .get(1)
            .is_some_and(|word| OF_WORD_PATTERN.matches_word(word))
        && OF_THOSE_OR_THEM_TAIL_PATTERN.matches_words(&all_words[1..])
        && let Some((count, used)) = parse_number(tokens)
        && used == 1
        && let Some(object_start) = token_word_view.token_index_for_word_index(3)
    {
        let object_tokens = trim_commas(&tokens[object_start..]);
        if !object_tokens.is_empty() {
            let other = object_tokens
                .first()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word));
            let mut filter = parse_object_filter(&object_tokens, other)?;
            filter =
                filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            let mut count = ChoiceCount::exactly(count as usize);
            if random_choice {
                count = count.at_random();
            }
            return Ok(wrap_target_count(
                TargetAst::Object(filter, None, span),
                Some(count),
            ));
        }
    }
    if IT_OR_THEM_WITH_PREFIX_PATTERN.matches_words(&all_words)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[2..])
        && consumed == all_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if TAGGED_OBJECT_TARGET_PATTERN.matches_words(&all_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if REST_TARGET_PATTERN.matches_words(&all_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("rest"), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if remaining_words.len() >= 2
        && CHOSEN_WORD_PATTERN.matches_word(remaining_words[0])
        && is_demonstrative_object_head(remaining_words[1])
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if EQUIPPED_OBJECT_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("equipped"), span),
            target_count,
        ));
    }
    if ENCHANTED_OBJECT_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("enchanted"), span),
            target_count,
        ));
    }
    if CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("tap_cost_0"), span),
            target_count,
        ));
    }

    if token_slice_at_is(tokens, idx, "any")
        && token_slice_at_is(tokens, idx + 1, "number")
        && token_slice_at_is(tokens, idx + 2, "of")
    {
        if let Some((count, used)) = parse_choice_count_token_prefix_consumed(&tokens[idx..]) {
            target_count = Some(count);
            idx += used;
        }
    }

    if token_slice_at_is(tokens, idx, "up") && token_slice_at_is(tokens, idx + 1, "to") {
        if let Some((count, used)) = parse_choice_count_token_prefix_consumed(&tokens[idx..]) {
            target_count = Some(count);
            idx += used;
        } else {
            let next_word = tokens
                .get(idx + 2)
                .and_then(OwnedLexToken::as_word)
                .unwrap_or("?");
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic or missing target count after 'up to' (found '{next_word}' in clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    } else if let Some((count, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    } else if let Some((value, used)) = parse_number_or_x_value(&tokens[idx..]) {
        let next_is_target = tokens
            .get(idx + used)
            .is_some_and(|token| TARGET_WORD_PATTERN.matches_token(token));
        let next_is_other_target = tokens
            .get(idx + used)
            .is_some_and(|token| OTHER_WORD_PATTERN.matches_token(token))
            && tokens
                .get(idx + used + 1)
                .is_some_and(|token| TARGET_WORD_PATTERN.matches_token(token));
        let object_selector_idx = target_count_object_selector_index(tokens, idx + used);
        let next_is_object_selector = tokens
            .get(object_selector_idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_target_count_object_selector);
        if next_is_target || next_is_other_target || next_is_object_selector {
            target_count = Some(choice_count_from_value(&value, false));
            idx += used;
        }
    }

    if random_choice {
        target_count = Some(target_count.unwrap_or_default().at_random());
    }

    if token_slice_at_is(tokens, idx, "on") {
        idx += 1;
    }

    while tokens
        .get(idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(is_article)
    {
        idx += 1;
    }

    let mut saw_top_prefix = false;
    if token_slice_at_is(tokens, idx, "top") {
        saw_top_prefix = true;
        let count_idx = idx + 1;

        if let Some((value, used)) = parse_number_or_x_value(&tokens[count_idx..]) {
            let object_selector_idx = target_count_object_selector_index(tokens, count_idx + used);
            let next_is_object_selector = tokens
                .get(object_selector_idx)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(is_target_count_object_selector);
            if next_is_object_selector {
                target_count = Some(choice_count_from_value(&value, false));
                idx = count_idx + used;
            }
        }
    }

    if token_slice_at_is(tokens, idx, "other") && token_slice_at_is(tokens, idx + 1, "target") {
        other = true;
        explicit_target = true;
        idx += 2;
    } else {
        if token_slice_at_is(tokens, idx, "another") || token_slice_at_is(tokens, idx, "other") {
            other = true;
            idx += 1;
        }

        if token_slice_at_is(tokens, idx, "target") {
            explicit_target = true;
            idx += 1;
        }
    }

    if let Some(ordinal_word) = tokens.get(idx).and_then(OwnedLexToken::as_word)
        && matches!(
            ordinal_word,
            "first"
                | "second"
                | "third"
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
        )
        && tokens
            .get(idx + 1)
            .is_some_and(|token| TARGET_WORD_PATTERN.matches_token(token))
    {
        if ordinal_word != "first" {
            other = true;
        }
        explicit_target = true;
        idx += 2;
    }

    let words_all = crate::runtime_backend::token_word_refs(&tokens[idx..]);
    if ANY_TARGET_PATTERN.matches_words(&words_all) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }
    if ANY_OTHER_TARGET_PATTERN.matches_words(&words_all) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = crate::runtime_backend::token_word_refs(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.is_empty() && explicit_target {
        return Ok(wrap_target_count(
            if other {
                TargetAst::AnyOtherTarget(span)
            } else {
                TargetAst::AnyTarget(span)
            },
            target_count,
        ));
    }
    if other && TARGET_OR_TARGETS_WORD_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }
    if TARGET_OR_TARGETS_WORD_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let bare_top_library_shorthand = saw_top_prefix
        && !remaining_words
            .iter()
            .any(|word| LIBRARY_WORD_PATTERN.matches_word(word))
        && (TOP_CARD_TARGET_SHORTHAND_PATTERN.matches_words(&remaining_words)
            || (target_count.is_some()
                && CARDS_TARGET_SHORTHAND_PATTERN.matches_words(&remaining_words)));
    if bare_top_library_shorthand {
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    if let Some(filter) = parse_hand_advantage_player_target_filter(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if let Some(filter) = parse_life_advantage_player_target_filter(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if PLAYER_ON_YOUR_TEAM_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if other && ANY_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::NotYou, target_span),
            target_count,
        ));
    }
    if ANY_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if ENCHANTED_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::TaggedPlayer(TagKey::from("enchanted")),
                target_span,
            ),
            target_count,
        ));
    }
    if THAT_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if CHOSEN_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::ChosenPlayer, target_span),
            target_count,
        ));
    }
    if THAT_OPPONENT_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    if DEFENDING_PLAYER_EDGE_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Defending, target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words.get(1).is_some_and(|word| {
        let normalized = strip_possessive_suffix(word);
        matches!(
            normalized,
            "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "spell"
                | "spells"
                | "source"
                | "sources"
                | "card"
                | "cards"
        ) || parse_card_type(normalized).is_some()
            || parse_subtype_word(normalized).is_some()
            || str_strip_suffix(normalized, "s").is_some_and(|singular| {
                parse_card_type(singular).is_some() || parse_subtype_word(singular).is_some()
            })
    });
    if remaining_words.len() >= 3
        && THAT_OR_THE_WORD_PATTERN.matches_word(remaining_words[0])
        && second_word_is_object_head
        && CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN.matches_word(remaining_words[2])
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[2]);
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if remaining_words.len() >= 5
        && THAT_WORD_PATTERN.matches_word(remaining_words[0])
        && second_word_is_object_head
        && OR_WORD_PATTERN.matches_word(remaining_words[2])
        && is_demonstrative_object_head(remaining_words[3])
        && CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN.matches_word(remaining_words[4])
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[4]);
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }
    if remaining_words.len() >= 2 {
        let object_head = strip_possessive_suffix(remaining_words[0]);
        if matches!(
            remaining_words[1],
            "controller" | "controllers" | "owner" | "owners"
        ) && (matches!(
            object_head,
            "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "spell"
                | "spells"
                | "source"
                | "sources"
                | "card"
                | "cards"
        ) || parse_card_type(object_head).is_some()
            || parse_subtype_word(object_head).is_some()
            || str_strip_suffix(object_head, "s").is_some_and(|singular| {
                parse_card_type(singular).is_some() || parse_subtype_word(singular).is_some()
            }))
        {
            let player = tagged_it_owner_or_controller_player_filter(remaining_words[1]);
            return Ok(wrap_target_count(
                TargetAst::Player(player, target_span),
                target_count,
            ));
        }
    }
    if ITS_OR_THEIR_OWNER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }

    if YOU_OR_YOUR_PREFIX_PATTERN.matches_words(&remaining_words) && remaining_words.len() == 1 {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    if OPPONENT_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    if SPELL_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Spell(target_span),
            target_count,
        ));
    }
    if TRIGGERING_SPELL_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("triggering"), span),
            target_count,
        ));
    }
    if TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("triggering_source"), span),
            target_count,
        ));
    }

    if IT_OR_THEM_WITH_PREFIX_PATTERN.matches_words(&remaining_words)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, span),
            target_count,
        ));
    }

    if let Some(surface) = source_reference_surface_for_words(&remaining_words)
        .or_else(|| this_source_surface_for_words(&remaining_words))
    {
        let source_span = target_span.or(span);
        record_source_reference_surface(source_span, surface);
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }
    if is_source_from_your_graveyard_words(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Graveyard);
        source_filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if SOURCE_PT_REFERENCE_PREFIX_PATTERN.matches_words(&remaining_words)
        || SOURCE_PT_REFERENCE_TARGET_PATTERN.matches_words(&remaining_words)
    {
        let source_span = target_span.or(span);
        record_source_reference_surface(
            source_span,
            SourceReferenceSurface::ThisPermanentType(remaining_words.join(" ")),
        );
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }

    if IT_INSTEAD_THIS_WAY_PREFIX_PATTERN.matches_words(&remaining_words)
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| INSTEAD_THIS_WAY_WORD_PATTERN.matches_word(word))
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if TOKEN_CREATED_THIS_WAY_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if ITSELF_TARGET_PATTERN.matches_words(&remaining_words) {
        record_source_reference_surface(
            span,
            SourceReferenceSurface::ThisPermanentType("itself".to_string()),
        );
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if HIM_OR_HER_TARGET_PATTERN.matches_words(&remaining_words) {
        record_source_reference_surface(
            span,
            SourceReferenceSurface::ThisPermanentType(remaining_words[0].to_string()),
        );
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if THEM_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if THAT_PLAYER_TARGET_PATTERN.matches_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let attacking_you_or_your_planeswalker = matches!(
        remaining_words.as_slice(),
        [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ]
    );
    if attacking_you_or_your_planeswalker {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Creature);
        filter.attacking = true;
        filter.controller = Some(PlayerFilter::Opponent);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    let opponent_or_planeswalker = matches!(
        remaining_words.as_slice(),
        ["opponent", "or", "planeswalker"]
            | ["opponents", "or", "planeswalkers"]
            | ["planeswalker", "or", "opponent"]
            | ["planeswalkers", "or", "opponents"]
    );
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let player_or_planeswalker_its_attacking = find_window_by(&remaining_words, 3, |window| {
        matches!(
            window,
            ["player", "or", "planeswalker"]
                | ["players", "or", "planeswalkers"]
                | ["planeswalker", "or", "player"]
                | ["planeswalkers", "or", "players"]
        )
    })
    .is_some()
        && remaining_words
            .iter()
            .any(|word| ATTACKING_WORD_PATTERN.matches_word(word))
        && IT_THAT_ATTACKING_REFERENCE_MARKER_PATTERN.matches_words(&remaining_words);
    if player_or_planeswalker_its_attacking {
        return Ok(wrap_target_count(
            TargetAst::AttackedPlayerOrPlaneswalker(target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = matches!(
        remaining_words.as_slice(),
        ["player", "or", "planeswalker"]
            | ["players", "or", "planeswalkers"]
            | ["planeswalker", "or", "player"]
            | ["planeswalkers", "or", "players"]
    );
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        remaining_words.as_slice(),
        ["permanent", "or", "player"]
            | ["permanents", "or", "players"]
            | ["player", "or", "permanent"]
            | ["players", "or", "permanents"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let creature_or_player = find_window_by(&remaining_words, 3, |window| {
        matches!(
            window,
            ["creature", "or", "player"]
                | ["creatures", "or", "players"]
                | ["player", "or", "creature"]
                | ["players", "or", "creatures"]
                | ["creature", "and", "player"]
                | ["creatures", "and", "players"]
                | ["player", "and", "creature"]
                | ["players", "and", "creatures"]
                | ["creature", "and/or", "player"]
                | ["creatures", "and/or", "players"]
                | ["player", "and/or", "creature"]
                | ["players", "and/or", "creatures"]
        )
    })
    .is_some()
        || find_window_by(&remaining_words, 4, |window| {
            matches!(
                window,
                ["creature", "and", "or", "player"]
                    | ["creatures", "and", "or", "players"]
                    | ["player", "and", "or", "creature"]
                    | ["players", "and", "or", "creatures"]
            )
        })
        .is_some();
    if creature_or_player {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let mixed_object_player_target =
        MIXED_PLAYER_PLANESWALKER_TOKEN_PATTERN.matches_words(&remaining_words);
    if mixed_object_player_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-token/player/planeswalker target phrase (clause: '{}')",
            remaining_words.join(" ")
        )));
    }

    let mut filter = parse_object_filter(remaining, other)?;
    if filter.with_counter.is_none()
        && remaining_words
            .first()
            .is_some_and(|word| IT_OR_THEM_WORD_PATTERN.matches_word(word))
        && remaining_words
            .get(1)
            .is_some_and(|word| WITH_WORD_PATTERN.matches_word(word))
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        filter.with_counter = Some(counter_constraint);
    }
    let it_span = if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mut idx = tokens.len();
        let mut found_span = None;
        while idx > 0 {
            idx -= 1;
            if IT_WORD_PATTERN.matches_token(&tokens[idx]) {
                found_span = Some(tokens[idx].span());
                break;
            }
        }
        found_span
    } else {
        None
    };
    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, it_span),
        target_count,
    ))
}

fn parse_hand_advantage_player_target_filter(words: &[&str]) -> Option<PlayerFilter> {
    let (base, mut idx) = match words.first().copied()? {
        "opponent" | "opponents" => (PlayerFilter::Opponent, 1),
        "player" | "players" => (PlayerFilter::Any, 1),
        _ => return None,
    };

    if !words
        .get(idx)
        .is_some_and(|word| WHO_OR_THAT_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;
    if !words
        .get(idx)
        .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;

    if words
        .get(idx..idx + 2)
        .is_some_and(|words| AT_LEAST_PATTERN.matches_words(words))
    {
        idx += 2;
    }

    let count = parse_number_word_u32(words.get(idx).copied()?)?;
    idx += 1;

    if !words
        .get(idx..idx + 3)
        .is_some_and(|words| MORE_CARD_IN_PATTERN.matches_words(words))
    {
        return None;
    }
    idx += 3;

    if words
        .get(idx)
        .is_some_and(|word| THEIR_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if !words
        .get(idx)
        .is_some_and(|word| HAND_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;

    if !words
        .get(idx..idx + 2)
        .is_some_and(|words| THAN_YOU_PATTERN.matches_words(words))
    {
        return None;
    }
    idx += 2;
    if words
        .get(idx)
        .is_some_and(|word| DO_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    if idx < words.len() {
        let rest = &words[idx..];
        if !AS_YOU_ACTIVATE_THIS_ABILITY_PATTERN.matches_words(rest) {
            return None;
        }
    }

    Some(PlayerFilter::CardsInHandAtLeastMoreThanYou {
        base: Box::new(base),
        count,
    })
}

fn parse_life_advantage_player_target_filter(words: &[&str]) -> Option<PlayerFilter> {
    let (base, mut idx) = match words.first().copied()? {
        "opponent" | "opponents" => (PlayerFilter::Opponent, 1),
        "player" | "players" => (PlayerFilter::Any, 1),
        _ => return None,
    };

    if !words
        .get(idx)
        .is_some_and(|word| WHO_OR_THAT_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;
    if !words
        .get(idx)
        .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;

    if !words
        .get(idx..idx + 4)
        .is_some_and(|words| MORE_LIFE_THAN_YOU_PATTERN.matches_words(words))
    {
        return None;
    }
    idx += 4;

    if words
        .get(idx)
        .is_some_and(|word| DO_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    if idx < words.len() {
        let rest = &words[idx..];
        if !AS_YOU_ACTIVATE_THIS_ABILITY_PATTERN.matches_words(rest) {
            return None;
        }
    }

    Some(PlayerFilter::HasMoreLifeThanYou {
        base: Box::new(base),
    })
}

pub(crate) fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, String)> {
    let tokens = lex_line(line.trim(), 0).ok()?;
    let dash_idx = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))?;
    let prefix_tokens = tokens.get(..dash_idx)?;
    let rest_tokens = tokens.get(dash_idx + 1..)?;

    let mut chapters = Vec::new();
    for roman in TokenWordView::new(prefix_tokens).word_refs() {
        chapters.push(roman_to_int(roman)?);
    }

    let rest = render_token_slice(rest_tokens).trim().to_string();
    (!chapters.is_empty() && !rest.is_empty()).then_some((chapters, rest))
}

fn roman_to_int(roman: &str) -> Option<u32> {
    match roman {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

pub(crate) fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let tokens = lex_line(line.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens);
    if !words.starts_with(&["level"]) {
        return None;
    }

    let range_start = words.token_index_after_words(1)?;
    let range_tokens = tokens.get(range_start..)?;
    if let Some(parsed) = parse_level_header_range_tokens(range_tokens) {
        return Some(parsed);
    }

    let range_word = words.get(1)?;
    parse_level_header_range_word(range_word)
}

fn parse_level_header_range_tokens(tokens: &[OwnedLexToken]) -> Option<(u32, Option<u32>)> {
    let first = tokens.first()?;
    let min = parse_u32_token(first)?;
    match tokens.get(1).map(|token| token.kind) {
        Some(TokenKind::Plus) => Some((min, None)),
        Some(TokenKind::Dash) => {
            let max = parse_u32_token(tokens.get(2)?)?;
            Some((min, Some(max)))
        }
        _ => Some((min, Some(min))),
    }
}

fn parse_level_header_range_word(word: &str) -> Option<(u32, Option<u32>)> {
    let mut chars = word.chars();
    let min = take_ascii_u32(&mut chars)?;
    match chars.next() {
        None => Some((min, Some(min))),
        Some('+') if chars.next().is_none() => Some((min, None)),
        Some('-') => {
            let max = take_ascii_u32(&mut chars)?;
            chars.next().is_none().then_some((min, Some(max)))
        }
        _ => None,
    }
}

fn parse_u32_token(token: &OwnedLexToken) -> Option<u32> {
    parse_level_header_range_word(token.parser_text())
        .and_then(|(min, max)| if max == Some(min) { Some(min) } else { None })
}

fn take_ascii_u32(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    let mut value = 0u32;
    let mut consumed = false;
    while let Some(ch) = chars.as_str().chars().next() {
        let Some(digit) = ch.to_digit(10) else {
            break;
        };
        consumed = true;
        value = value.checked_mul(10)?.checked_add(digit)?;
        chars.next();
    }
    consumed.then_some(value)
}

pub(crate) fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let trimmed = raw.trim();
    let (power_text, toughness_text) = trimmed.split_once('/')?;

    let power = parse_pt_value(power_text)?;
    let toughness = parse_pt_value(toughness_text)?;
    Some(PowerToughness::new(power, toughness))
}

fn parse_pt_value(raw: &str) -> Option<PtValue> {
    let raw = raw.trim();
    if char_sequence_eq(raw, &['.', '5']) || char_sequence_eq(raw, &['0', '.', '5']) {
        return Some(PtValue::Fixed(0));
    }
    if char_sequence_eq(raw, &['*']) {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = strip_char_prefix_sequence(raw, &['*', '+']) {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = strip_char_suffix_sequence(raw, &['+', '*']) {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    raw.parse::<i32>().ok().map(PtValue::Fixed)
}

fn char_sequence_eq(text: &str, expected: &[char]) -> bool {
    let mut chars = text.chars();
    expected
        .iter()
        .all(|expected| chars.next().is_some_and(|ch| ch == *expected))
        && chars.next().is_none()
}

fn strip_char_prefix_sequence<'a>(text: &'a str, expected: &[char]) -> Option<&'a str> {
    let mut rest = text;
    for expected in expected {
        let mut chars = rest.chars();
        if chars.next()? != *expected {
            return None;
        }
        rest = chars.as_str();
    }
    Some(rest)
}

fn strip_char_suffix_sequence<'a>(text: &'a str, expected: &[char]) -> Option<&'a str> {
    let mut end = text.len();
    for expected in expected.iter().rev() {
        let head = text.get(..end)?;
        let ch = head.chars().next_back()?;
        if ch != *expected {
            return None;
        }
        end = end.saturating_sub(ch.len_utf8());
    }
    text.get(..end)
}

pub(crate) fn parse_level_up_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = UtilWordView::new(tokens);
    if !word_view.slice_eq(0, LEVEL_UP_PREFIX_WORDS) {
        return Ok(None);
    }

    let (mana_cost, _) = leading_mana_cost_from_tokens(tokens.get(2..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("level up missing mana cost".to_string()))?;
    let level_up_text = format!("Level up {}", mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters_on_source(CounterType::Level, 1),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(level_up_text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_level_up_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_level_up_line(tokens)
}

pub(crate) fn preserve_keyword_prefix_for_parse(prefix: &str) -> bool {
    let Some(tokens) = lex_line(prefix.trim(), 0).ok() else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    let Some(first) = words.first().copied() else {
        return false;
    };

    matches!(
        first,
        "buyback"
            | "blitz"
            | "bestow"
            | "cumulative"
            | "cycling"
            | "echo"
            | "equip"
            | "epic"
            | "escape"
            | "escalate"
            | "eternalize"
            | "evoke"
            | "flashback"
            | "kicker"
            | "multikicker"
            | "boast"
            | "modular"
            | "morph"
            | "megamorph"
            | "replicate"
            | "reinforce"
            | "renew"
            | "squad"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

pub(crate) fn parse_self_free_cast_alternative_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    let clause_word_view = UtilWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !SELF_FREE_CAST_ALTERNATIVE_COST_PATTERN.matches_words(&clause_words) {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub(crate) fn parse_self_free_cast_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_self_free_cast_alternative_cost_line(tokens)
}

pub(crate) fn parse_flash_with_additional_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    let clause_word_view = UtilWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !FLASH_WITH_ADDITIONAL_COST_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }
    let cost_start = token_index_for_word_index(tokens, 13)?;
    let (additional_cost, used) = leading_mana_cost_from_tokens(&tokens[cost_start..])?;
    let suffix_words = UtilWordView::new(&tokens[cost_start + used..]).to_word_refs();
    if !FLASH_WITH_ADDITIONAL_COST_SUFFIX_PATTERN.matches_words(&suffix_words) {
        return None;
    }
    Some(AlternativeCastingMethod::flash_with_additional_cost(
        additional_cost,
        crate::cost::TotalCost::free(),
    ))
}

pub(crate) fn parse_flash_with_additional_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_flash_with_additional_cost_line(tokens)
}

fn leading_mana_symbols_to_oracle(words_all: &[&str]) -> Option<(String, usize)> {
    let mut symbols = Vec::new();
    let mut consumed = 0usize;
    for word in words_all {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        symbols.push(symbol);
        consumed += 1;
    }
    if symbols.is_empty() {
        return None;
    }
    Some((ManaCost::from_symbols(symbols).to_oracle(), consumed))
}

pub(crate) fn mana_pips_from_token(token: &OwnedLexToken) -> Option<Vec<ManaSymbol>> {
    match token.kind {
        TokenKind::Word | TokenKind::Number => parse_mana_symbol(token.slice.as_str())
            .ok()
            .map(|symbol| vec![symbol]),
        TokenKind::ManaGroup => {
            let inner = token.mana_group_inner()?;
            if inner.is_empty() {
                return None;
            }
            parse_mana_symbol_group(inner)
                .ok()
                .filter(|group| !group.is_empty())
        }
        _ => None,
    }
}

pub(crate) fn leading_mana_cost_from_tokens(tokens: &[OwnedLexToken]) -> Option<(ManaCost, usize)> {
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for token in tokens {
        let Some(group) = mana_pips_from_token(token) else {
            break;
        };
        pips.push(group);
        consumed += 1;
    }
    if pips.is_empty() {
        return None;
    }
    Some((ManaCost::from_pips(pips), consumed))
}

pub(crate) fn parse_madness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "madness") {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let cost_end = find_token_kind(cost_tokens, TokenKind::Comma).unwrap_or(cost_tokens.len());
    let cost_tokens = &cost_tokens[..cost_end];
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("madness keyword missing mana symbols".to_string())
    })?;

    Ok(Some(AlternativeCastingMethod::Madness { cost: mana_cost }))
}

pub(crate) fn parse_madness_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_madness_line(tokens)
}

pub(crate) fn parse_buyback_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    if !token_slice_first_is(tokens, "buyback") {
        return Ok(None);
    }

    if token_slice_at_is(tokens, 1, "costs") {
        return Ok(None);
    }

    let tail = tokens.get(1..).unwrap_or_default();
    if tail.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let reminder_start = find_window_by(tail, 3, |window| {
        token_slice_starts_with(window, &["you", "may", "pay"])
    })
    .or_else(|| {
        find_window_by(tail, 2, |window| {
            token_slice_starts_with(window, &["you", "may"])
        })
    })
    .unwrap_or(tail.len());
    let cost_tokens = trim_commas(&tail[..reminder_start]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(OptionalCost::buyback(total_cost)))
}

pub(crate) fn parse_buyback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_buyback_line(tokens)
}

pub(crate) fn parse_bargain_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    let clause_view = UtilWordView::new(tokens);
    let clause_words = clause_view.to_word_refs();
    if !clause_words
        .first()
        .is_some_and(|word| BARGAIN_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let filter = crate::target::ObjectFilter {
        zone: Some(crate::zone::Zone::Battlefield),
        controller: Some(crate::target::PlayerFilter::You),
        any_of: vec![
            crate::target::ObjectFilter::artifact(),
            crate::target::ObjectFilter::enchantment(),
            crate::target::ObjectFilter::default().token(),
        ],
        ..Default::default()
    };

    Ok(Some(OptionalCost::custom(
        "Bargain",
        TotalCost::from_cost(Cost::sacrifice(filter)),
    )))
}

pub(crate) fn parse_bargain_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_bargain_line(tokens)
}

pub(crate) fn parse_optional_cost_keyword_line(
    tokens: &[OwnedLexToken],
    keyword: &str,
    constructor: fn(TotalCost) -> OptionalCost,
) -> Result<Option<OptionalCost>, CardTextError> {
    if !token_slice_first_is(tokens, keyword) {
        return Ok(None);
    }

    let mut tail = tokens.get(1..).unwrap_or_default();
    if matches!(
        tail.first().map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) {
        tail = tail.get(1..).unwrap_or_default();
    }
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let reminder_start = find_window_by(tail, 3, |window| {
        token_slice_starts_with(window, &["you", "may", "pay"])
    })
    .or_else(|| {
        find_window_by(tail, 2, |window| {
            token_slice_starts_with(window, &["you", "may"])
        })
    })
    .unwrap_or(tail.len());
    let sentence_end = find_token_kind(tail, TokenKind::Period).unwrap_or(tail.len());
    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(constructor(total_cost)))
}

pub(crate) fn parse_kicker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "kicker", OptionalCost::kicker)
}

pub(crate) fn parse_kicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_kicker_line(tokens)
}

pub(crate) fn parse_multikicker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "multikicker", OptionalCost::multikicker)
}

pub(crate) fn parse_multikicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_multikicker_line(tokens)
}

pub(crate) fn parse_replicate_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| REPLICATE_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let mut tail = tokens.get(1..).unwrap_or_default();
    if matches!(
        tail.first().map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) {
        tail = tail.get(1..).unwrap_or_default();
    }
    if tail.is_empty() {
        return Err(CardTextError::ParseError(
            "replicate keyword missing cost".to_string(),
        ));
    }

    let reminder_start = find_token_kind(tail, TokenKind::LParen).unwrap_or(tail.len());
    let sentence_end = find_token_kind(tail, TokenKind::Period).unwrap_or(tail.len());
    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "replicate keyword missing cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(OptionalCost::replicate(total_cost)))
}

pub(crate) fn parse_replicate_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_replicate_line(tokens)
}

pub(crate) fn parse_squad_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "squad", OptionalCost::squad)
}

pub(crate) fn parse_squad_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_squad_line(tokens)
}

pub(crate) fn parse_offspring_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "offspring", OptionalCost::offspring)
}

pub(crate) fn parse_offspring_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_offspring_line(tokens)
}

pub(crate) fn parse_entwine_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "entwine", OptionalCost::entwine)
}

pub(crate) fn parse_entwine_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_entwine_line(tokens)
}

fn keyword_cost_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    keyword: &str,
) -> Option<Vec<OwnedLexToken>> {
    if !token_slice_first_is(tokens, keyword) {
        return None;
    }

    let mut tail = tokens.get(1..).unwrap_or_default();
    if matches!(
        tail.first().map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) {
        tail = tail.get(1..).unwrap_or_default();
    }

    let reminder_start = find_token_kind(tail, TokenKind::LParen).unwrap_or(tail.len());
    let sentence_end = find_token_kind(tail, TokenKind::Period).unwrap_or(tail.len());
    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    (!cost_tokens.is_empty()).then_some(cost_tokens)
}

pub(crate) fn parse_escalate_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(TotalCost, String)>, CardTextError> {
    let Some(cost_tokens) = keyword_cost_tail_tokens(tokens, "escalate") else {
        return Ok(None);
    };
    let total_cost = parse_activation_cost(&cost_tokens)?;
    let display = render_token_slice(&cost_tokens).trim().to_string();
    Ok(Some((total_cost, display)))
}

pub(crate) fn parse_evoke_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(cost_tokens) = keyword_cost_tail_tokens(tokens, "evoke") else {
        return Ok(None);
    };
    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Evoke",
        total_cost,
        condition: None,
    }))
}

pub(crate) fn parse_prowl_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(cost_tokens) = keyword_cost_tail_tokens(tokens, "prowl") else {
        return Ok(None);
    };
    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Prowl",
        total_cost,
        condition: Some(
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue,
            ),
        ),
    }))
}

pub(crate) fn parse_eternalize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ManaCost>, CardTextError> {
    let Some(cost_tokens) = keyword_cost_tail_tokens(tokens, "eternalize") else {
        return Ok(None);
    };
    let (mana_cost, consumed) = leading_mana_cost_from_tokens(&cost_tokens).ok_or_else(|| {
        CardTextError::ParseError("eternalize keyword missing mana cost".to_string())
    })?;
    if consumed == 0 {
        return Err(CardTextError::ParseError(
            "eternalize keyword missing mana cost".to_string(),
        ));
    }
    Ok(Some(mana_cost))
}

pub(crate) fn parse_epic_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    token_slice_first_is(tokens, "epic")
}

pub(crate) fn parse_morph_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = UtilWordView::new(tokens);
    let Some(first_word) = word_view.first() else {
        return Ok(None);
    };

    let (is_megamorph, is_disguise) = match first_word {
        "morph" => (false, false),
        "megamorph" => (true, false),
        "disguise" => (false, true),
        _ => return Ok(None),
    };

    let tail = tokens.get(1..).unwrap_or_default();
    if tail.is_empty() {
        let mechanic = if is_disguise {
            "disguise"
        } else if is_megamorph {
            "megamorph"
        } else {
            "morph"
        };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing cost"
        )));
    }

    let reminder_start = find_window_by(tail, 3, |window| {
        token_slice_starts_with(window, &["you", "may", "cast"])
    })
    .or_else(|| {
        find_window_by(tail, 4, |window| {
            token_slice_starts_with(window, &["turn", "it", "face", "up"])
        })
    })
    .unwrap_or(tail.len());
    let sentence_end = find_token_kind(tail, TokenKind::Period).unwrap_or(tail.len());
    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    if cost_tokens.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing cost"
        )));
    }

    let unsupported_cost_clause = || {
        let mechanic = if is_disguise {
            "disguise"
        } else if is_megamorph {
            "megamorph"
        } else {
            "morph"
        };
        CardTextError::ParseError(format!(
            "unsupported {mechanic} cost clause (line: '{}')",
            render_token_slice(&cost_tokens).trim()
        ))
    };
    let cost = match parse_activation_cost(&cost_tokens) {
        Ok(cost) if !cost.is_free() => cost,
        _ if leading_mana_cost_from_tokens(&cost_tokens).is_some() => {
            return Err(unsupported_cost_clause());
        }
        _ => {
            crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(&cost_tokens)?
                .ok_or_else(unsupported_cost_clause)?
        }
    };
    if cost.is_free() {
        let mechanic = if is_disguise {
            "disguise"
        } else if is_megamorph {
            "megamorph"
        } else {
            "morph"
        };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing cost"
        )));
    }

    let label = if is_disguise {
        "Disguise"
    } else if is_megamorph {
        "Megamorph"
    } else {
        "Morph"
    };
    let text = format!("{label}—{}", cost.display());
    let static_ability = if is_disguise {
        StaticAbility::disguise(cost)
    } else if is_megamorph {
        StaticAbility::megamorph(cost)
    } else {
        StaticAbility::morph(cost)
    };

    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(static_ability).into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_morph_keyword_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_morph_keyword_line(tokens)
}

pub(crate) fn parse_escape_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "escape") {
        return Ok(None);
    }

    let cost_start = 1usize;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let comma_idx = find_token_kind(&tokens[cost_start..], TokenKind::Comma)
        .map(|idx| cost_start + idx)
        .ok_or_else(|| {
            CardTextError::ParseError("escape keyword missing exile clause separator".to_string())
        })?;
    if comma_idx <= cost_start {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&tokens[cost_start..comma_idx])?;
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing mana symbols".to_string())
    })?;

    let tail_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword missing exile clause".to_string(),
        ));
    }

    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !tail_words
        .first()
        .is_some_and(|word| ESCAPE_EXILE_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    let Some((exile_count, used)) = parse_number_or_x_value(&tail_tokens[1..]) else {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exile count (clause: '{}')",
            tail_words.join(" ")
        )));
    };
    let Value::Fixed(exile_count) = exile_count else {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape exile count (clause: '{}')",
            tail_words.join(" ")
        )));
    };
    let mut idx = 1 + used;
    if tail_words
        .get(idx)
        .is_some_and(|word| OTHER_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if !tail_words
        .get(idx)
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exiled card noun (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    idx += 1;
    if !tail_words
        .get(idx..idx + 3)
        .is_some_and(|words| FROM_YOUR_GRAVEYARD_PATTERN.matches_words(words))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    if idx + 3 != tail_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing escape clause segment (clause: '{}')",
            tail_words.join(" ")
        )));
    }

    Ok(Some(AlternativeCastingMethod::Escape {
        cost: Some(mana_cost),
        exile_count: exile_count as u32,
    }))
}

pub(crate) fn parse_escape_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_escape_line(tokens)
}

pub(crate) fn parse_flashback_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| FLASHBACK_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;

    Ok(Some(AlternativeCastingMethod::Flashback { total_cost }))
}

pub(crate) fn parse_flashback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_flashback_line(tokens)
}

pub(crate) fn parse_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "retrace") {
        return Ok(None);
    }

    Ok(Some(AlternativeCastingMethod::Retrace {
        total_cost: TotalCost::from_cost(Cost::discard(1, Some(CardType::Land))),
    }))
}

pub(crate) fn parse_retrace_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_retrace_line(tokens)
}

pub(crate) fn parse_jump_start_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let parser_words = words(tokens);
    if parser_words
        .first()
        .is_some_and(|word| JUMP_START_WORD_PATTERN.matches_word(word))
        || JUMP_START_SPLIT_PATTERN.matches_words(&parser_words)
    {
        return Ok(Some(AlternativeCastingMethod::JumpStart));
    }
    Ok(None)
}

pub(crate) fn parse_jump_start_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_jump_start_line(tokens)
}

pub(crate) fn parse_harmonize_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "harmonize") {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "harmonize keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "harmonize keyword missing mana symbols".to_string(),
        ));
    }

    Ok(Some(AlternativeCastingMethod::Harmonize { total_cost }))
}

pub(crate) fn parse_harmonize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_harmonize_line(tokens)
}

pub(crate) fn parse_warp_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "warp") {
        return Ok(None);
    }

    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("warp keyword missing mana cost".to_string()))?;
    Ok(Some(AlternativeCastingMethod::Warp { cost }))
}

pub(crate) fn parse_warp_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_warp_line(tokens)
}

pub(crate) fn parse_bestow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "bestow") {
        return Ok(None);
    }

    let (mana_cost, consumed_mana_tokens) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("bestow keyword missing mana cost".to_string())
        })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());
    let consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));

    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if token_slice_first_kind(tail_tokens, TokenKind::Comma) {
        let clause_end =
            find_token_kind(tail_tokens, TokenKind::Period).unwrap_or(tail_tokens.len());
        let clause_tokens = trim_commas(&tail_tokens[..clause_end]).to_vec();
        let clause_words = crate::runtime_backend::token_word_refs(&clause_tokens);
        if let Some(first_word) = clause_words.first()
            && !IF_WORD_PATTERN.matches_words(&[*first_word])
        {
            cost_tokens.extend(clause_tokens);
        }
    }

    if let Ok(parsed_total_cost) = parse_activation_cost(&cost_tokens) {
        total_cost = parsed_total_cost;
        if total_cost.mana_cost().is_none() {
            let mut components = total_cost.costs().to_vec();
            components.insert(0, crate::costs::Cost::mana(mana_cost));
            total_cost = TotalCost::from_costs(components);
        }
    }

    Ok(Some(AlternativeCastingMethod::Bestow { total_cost }))
}

pub(crate) fn parse_bestow_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_bestow_line(tokens)
}

pub(crate) fn parse_blitz_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !token_slice_first_is(tokens, "blitz") {
        return Ok(None);
    }

    let (mana_cost, consumed_mana_tokens) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("blitz keyword missing mana cost".to_string())
        })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());
    let consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));
    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if token_slice_first_kind(tail_tokens, TokenKind::Comma) {
        let clause_end =
            find_token_kind(tail_tokens, TokenKind::Period).unwrap_or(tail_tokens.len());
        cost_tokens.extend(tail_tokens[..clause_end].iter().cloned());
    }
    if let Ok(parsed_total_cost) = parse_activation_cost(&cost_tokens) {
        total_cost = parsed_total_cost;
        if total_cost.mana_cost().is_none() {
            let mut components = total_cost.costs().to_vec();
            components.insert(0, crate::costs::Cost::mana(mana_cost));
            total_cost = TotalCost::from_costs(components);
        }
    }
    let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
    if let Some(pay_idx) = tail_words
        .iter()
        .position(|word| PAY_WORD_PATTERN.matches_word(word))
        && tail_words
            .get(pay_idx + 2)
            .is_some_and(|word| LIFE_WORD_PATTERN.matches_word(word))
        && !total_cost
            .costs()
            .iter()
            .any(|cost| matches!(cost, Cost::Life(_)))
        && let Some(amount_word) = tail_words.get(pay_idx + 1)
        && let Some(amount) = parse_number_word_u32(amount_word)
    {
        let mut components = total_cost.costs().to_vec();
        components.push(Cost::life(Value::Fixed(amount as i32)));
        total_cost = TotalCost::from_costs(components);
    }
    Ok(Some(AlternativeCastingMethod::Blitz { total_cost }))
}

pub(crate) fn parse_blitz_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_blitz_line(tokens)
}

pub(crate) fn parse_transmute_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = UtilWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    if !words_all
        .first()
        .is_some_and(|word| TRANSMUTE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let Some((base_mana_cost, _consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "transmute keyword missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let base_cost = TotalCost::mana(base_mana_cost.clone());
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut parsed_mana_value: Option<u32> = None;
    for idx in 0..word_view.len().saturating_sub(2) {
        if word_view.slice_eq(idx, MANA_VALUE_WORDS) {
            let start = word_view
                .token_index_for_word_index(idx + 2)
                .unwrap_or(tokens.len());
            parsed_mana_value =
                parse_number_or_x_value(&tokens[start..]).and_then(|(value, _)| match value {
                    Value::Fixed(n) if n >= 0 => Some(n as u32),
                    _ => None,
                });
            if parsed_mana_value.is_some() {
                break;
            }
        }
    }
    let filter = if let Some(mana_value) = parsed_mana_value {
        ObjectFilter::default().with_mana_value(crate::filter::Comparison::Equal(mana_value as i32))
    } else {
        ObjectFilter::default().with_mana_value(crate::filter::Comparison::EqualExpr(Box::new(
            crate::effect::Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source)),
        )))
    };
    let text = format!("Transmute {}", base_mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::search_library_to_hand(filter, true),
                ]),
                choices: Vec::new(),
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        }
        .into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_transmute_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_transmute_line(tokens)
}

pub(crate) fn parse_reinforce_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_view = UtilWordView::new(tokens);
    let words_all = words_view.to_word_refs();
    if !words_all
        .first()
        .is_some_and(|word| REINFORCE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let Some((amount_value, used_amount)) =
        parse_number_or_x_value(tokens.get(1..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing counter amount (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let Value::Fixed(amount) = amount_value else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reinforce amount (clause: '{}')",
            words_all.join(" ")
        )));
    };

    let cost_start = 1 + used_amount;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let Some((base_mana_cost, _consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(tokens.get(cost_start..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana symbols (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let base_cost = TotalCost::mana(base_mana_cost.clone());
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut creature_filter = ObjectFilter::default();
    creature_filter.zone = Some(Zone::Battlefield);
    creature_filter.card_types.push(CardType::Creature);

    let target = ChooseSpec::target(ChooseSpec::Object(creature_filter));
    let effect = Effect::put_counters(CounterType::PlusOnePlusOne, amount, target);

    let cost_text = base_mana_cost.to_oracle();
    let render_text = format!("Reinforce {amount} {cost_text}");

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        }
        .into(),
        text: Some(render_text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_reinforce_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_reinforce_line(tokens)
}

pub(crate) fn parse_cast_this_spell_only_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_word_view = UtilWordView::new(tokens);
    let line_words = line_word_view.to_word_refs();
    if !CAST_THIS_SPELL_ONLY_PREFIX_PATTERN.matches_words(line_words.as_slice()) {
        return Ok(None);
    }

    let tail = &line_words[4..];
    if CAST_ONLY_NO_PERMANENTS_NAMED_PREFIX_PATTERN.matches_words(tail) && tail.len() > 8 {
        let name_words = &tail[4..tail.len() - 4];
        let card_name = title_case_card_name_words(name_words);
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_no_permanents_named_on_battlefield(
                card_name.as_str(),
            ),
            format!(
                "Cast this spell only if no permanents named {card_name} are on the battlefield."
            ),
        )));
    }

    if let Some((kind, text)) = parse_cast_restriction_you_control_or_more_tail(tail) {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(kind, text)));
    }

    let restriction = if CAST_ONLY_DECLARE_ATTACKERS_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step(),
            "Cast this spell only during the declare attackers step.",
        ))
    } else if CAST_ONLY_DECLARE_ATTACKERS_IF_ATTACKED_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step(),
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
        ))
    } else if CAST_ONLY_DURING_COMBAT_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat(),
            "Cast this spell only during combat.",
        ))
    } else if CAST_ONLY_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_before_blockers_are_declared(),
            "Cast this spell only during combat before blockers are declared.",
        ))
    } else if CAST_ONLY_COMBAT_AFTER_BLOCKERS_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_after_blockers_are_declared(),
            "Cast this spell only during combat after blockers are declared.",
        ))
    } else if CAST_ONLY_YOUR_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_your_turn_before_blockers_are_declared(),
            "Cast this spell only during combat on your turn before blockers are declared.",
        ))
    } else if CAST_ONLY_OPPONENT_COMBAT_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_opponents_turn(
            ),
            "Cast this spell only during combat on an opponent's turn.",
        ))
    } else if CAST_ONLY_BEFORE_ATTACKERS_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::before_attackers_are_declared(),
            "Cast this spell only before attackers are declared.",
        ))
    } else if CAST_ONLY_BEFORE_COMBAT_DAMAGE_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::before_combat_damage_step(),
            "Cast this spell only before the combat damage step.",
        ))
    } else if CAST_ONLY_OPPONENTS_UPKEEP_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_upkeep(),
            "Cast this spell only during an opponent's upkeep.",
        ))
    } else if CAST_ONLY_OPPONENT_TURN_AFTER_UPKEEP_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_turn_after_upkeep(),
            "Cast this spell only during an opponent's turn after their upkeep step.",
        ))
    } else if CAST_ONLY_YOUR_END_STEP_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_your_end_step(),
            "Cast this spell only during your end step.",
        ))
    } else if CAST_ONLY_CAST_ANOTHER_SPELL_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_spell_this_turn(),
            "Cast this spell only if you've cast another spell this turn.",
        ))
    } else if CAST_ONLY_CAST_ANOTHER_GREEN_SPELL_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_green_spell_this_turn(),
            "Cast this spell only if you've cast another green spell this turn.",
        ))
    } else if CAST_ONLY_OPPONENT_CAST_CREATURE_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_opponent_cast_creature_spell_this_turn(),
            "Cast this spell only if an opponent cast a creature spell this turn.",
        ))
    } else if CAST_ONLY_CREATURE_ATTACKING_YOU_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_creature_is_attacking_you(),
            "Cast this spell only if a creature is attacking you.",
        ))
    } else if CAST_ONLY_AFTER_COMBAT_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::after_combat(),
            "Cast this spell only after combat.",
        ))
    } else if CAST_ONLY_CONTROL_SNOW_LAND_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_snow_land(),
            "Cast this spell only if you control a snow land.",
        ))
    } else if CAST_ONLY_FEWER_CREATURES_THAN_EACH_OPPONENT_TAIL_PATTERN.matches_words(tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_fewer_creatures_than_each_opponent(),
            "Cast this spell only if you control fewer creatures than each opponent.",
        ))
    } else {
        None
    };

    Ok(restriction.map(|(kind, text)| StaticAbility::this_spell_cast_restriction(kind, text)))
}

fn parse_cast_restriction_you_control_or_more_tail(
    tail: &[&str],
) -> Option<(
    crate::static_abilities::ThisSpellCastRestrictionKind,
    String,
)> {
    if tail.len() < 5 || !CAST_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN.matches_words(tail) {
        return None;
    }
    let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&tail[3..]);
    let (count, used) = parse_greater_than_or_equal_quantity_prefix(
        &quantity_tokens,
        false,
        false,
        "cast restriction",
    )
    .ok()
    .flatten()?;
    let filter_words = tail.get(3 + used..)?;
    if filter_words.is_empty() {
        return None;
    }
    let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    let subtype = cast_restriction_single_subtype_filter(&filter)?;
    let count_words = tail.get(3..3 + used)?;
    Some((
        crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
            subtype, count,
        ),
        format!(
            "Cast this spell only if you control {} {}.",
            count_words.join(" "),
            title_case_card_name_words(filter_words)
        ),
    ))
}

fn cast_restriction_single_subtype_filter(filter: &ObjectFilter) -> Option<Subtype> {
    if filter.subtypes.len() == 1
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && !filter.colorless
        && !filter.multicolored
        && !filter.monocolored
        && !filter.token
        && !filter.nontoken
        && !filter.tapped
        && !filter.untapped
        && !filter.other
    {
        return filter.subtypes.first().copied();
    }
    None
}

pub(crate) fn parse_cast_this_spell_only_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    parse_cast_this_spell_only_line(tokens)
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !(token_slice_first_is(tokens, "you") && token_slice_at_is(tokens, 1, "may")) {
        return Ok(None);
    }
    let Some(rather_idx) = find_index(tokens, |token| RATHER_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let rather_tail_view = UtilWordView::new(tokens.get(rather_idx + 1..).unwrap_or_default());
    let rather_tail = rather_tail_view.to_word_refs();
    if !RATHER_THAN_THIS_SPELL_COST_TAIL_PATTERN.matches_words(&rather_tail) {
        return Ok(None);
    }
    let cost_clause_end = (rather_idx + 1..tokens.len())
        .rfind(|idx| {
            tokens[*idx]
                .as_word()
                .is_some_and(|word| COST_OR_COSTS_WORD_PATTERN.matches_word(word))
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    let trailing_words = crate::runtime_backend::token_word_refs(&tokens[cost_clause_end + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            trailing_words.join(" ")
        )));
    }
    let cost_tokens = tokens.get(2..rather_idx).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let total_cost = crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(cost_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported alternative cost clause (line: '{}', cost: '{}')",
                line,
                render_token_slice(cost_tokens).trim()
            ))
        })?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost",
        total_cost,
        condition: None,
    }))
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_you_may_rather_than_spell_cost_line(tokens, line)
}

#[cfg(not(test))]
pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    fn render_option_text(tokens: &[OwnedLexToken]) -> String {
        tokens
            .iter()
            .filter(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
            .map(OwnedLexToken::parser_text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    let clause_view = UtilWordView::new(tokens);
    let clause_words = clause_view.to_word_refs();
    if ONE_OR_MORE_PATTERN
        .find_exact_window(clause_words.as_slice(), 3)
        .is_some()
    {
        return Ok(None);
    }
    if !OR_MARKER_PATTERN.matches_words(clause_words.as_slice()) {
        return Ok(None);
    }

    let option_tokens = split_lexed_slices_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut normalized_options = Vec::new();
    for mut option in option_tokens.into_iter().map(|option| option.to_vec()) {
        while option
            .first()
            .is_some_and(|token| AND_OR_WORD_PATTERN.matches_token(token))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }
        normalized_options.push(option);
    }

    if normalized_options.len() < 2 {
        return Ok(None);
    }

    if normalized_options
        .iter()
        .any(|option| find_verb(option).is_none() && !token_slice_first_is(option, "behold"))
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    for option in normalized_options {
        let effects = parse_effect_sentences_lexed(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                render_option_text(&option)
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: render_option_text(&option),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

#[cfg(test)]
pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    fn render_option_text(tokens: &[OwnedLexToken]) -> String {
        tokens
            .iter()
            .filter(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
            .map(OwnedLexToken::parser_text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    let clause_view = UtilWordView::new(tokens);
    let clause_words = clause_view.to_word_refs();
    if ONE_OR_MORE_PATTERN
        .find_exact_window(clause_words.as_slice(), 3)
        .is_some()
    {
        return Ok(None);
    }
    if !OR_MARKER_PATTERN.matches_words(clause_words.as_slice()) {
        return Ok(None);
    }

    let option_tokens = split_lexed_slices_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut normalized_options = Vec::new();
    for mut option in option_tokens.into_iter().map(|option| option.to_vec()) {
        while option
            .first()
            .is_some_and(|token| AND_OR_WORD_PATTERN.matches_token(token))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }
        normalized_options.push(option);
    }

    if normalized_options.len() < 2 {
        return Ok(None);
    }

    if normalized_options
        .iter()
        .any(|option| find_verb(option).is_none() && !token_slice_first_is(option, "behold"))
    {
        return Ok(None);
    }

    let mut options: Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>> =
        Vec::new();
    for option in normalized_options {
        let effects = parse_effect_sentences_lexed(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                render_option_text(&option)
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: render_option_text(&option),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

#[cfg(not(test))]
pub(crate) fn parse_additional_cost_choice_options_lexed(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    parse_additional_cost_choice_options(tokens)
}

#[cfg(test)]
pub(crate) fn parse_additional_cost_choice_options_lexed(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    parse_additional_cost_choice_options(tokens)
}

fn trap_condition_from_this_spell_cost_condition(
    condition: &crate::static_abilities::ThisSpellCostCondition,
) -> Option<crate::TrapCondition> {
    match condition {
        crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(
            count,
        ) => Some(crate::TrapCondition::OpponentCastSpells { count: *count }),
        crate::static_abilities::ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(
            _,
        ) => Some(crate::TrapCondition::CreatureDealtDamageToYou),
        _ => None,
    }
}

fn simple_trap_cost_from_alternative_method(method: &AlternativeCastingMethod) -> Option<ManaCost> {
    let AlternativeCastingMethod::Composed { total_cost, .. } = method else {
        return None;
    };
    if total_cost.non_mana_costs().next().is_some() {
        return None;
    }
    Some(
        total_cost
            .mana_cost()
            .cloned()
            .unwrap_or_else(ManaCost::new),
    )
}

pub(crate) fn parse_if_conditional_alternative_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let clause_word_view = UtilWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !IF_WORD_PATTERN.matches_words(clause_words.as_slice()) {
        return Ok(None);
    }

    let (condition_tokens, tail_tokens) =
        if let Some(comma_idx) = find_index(tokens, |token| token.is_comma()) {
            (
                trim_commas(&tokens[1..comma_idx]),
                trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default()),
            )
        } else if let Some(may_idx) = find_window_by(tokens, 3, |window| {
            token_slice_starts_with(window, &["you", "may", "pay"])
        }) {
            (
                trim_commas(&tokens[1..may_idx]),
                trim_commas(&tokens[may_idx..]),
            )
        } else {
            return Ok(None);
        };
    if parse_self_free_cast_alternative_cost_line(&tail_tokens).is_none()
        && parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }
    let condition = if let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) {
        condition
    } else {
        let condition_words_view = UtilWordView::new(&condition_tokens);
        let condition_words = condition_words_view.to_word_refs();
        if FREERUNNING_ASSASSIN_OR_COMMANDER_CONDITION_PATTERN
            .matches_words(condition_words.as_slice())
        {
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                crate::types::Subtype::Assassin,
            )
        } else if DEALT_DAMAGE_BY_CREATURES_CONDITION_PATTERN
            .matches_words(condition_words.as_slice())
        {
            let count_start = if condition_words
                .first()
                .is_some_and(|word| YOUVE_WORD_PATTERN.matches_word(word))
            {
                5usize
            } else {
                6usize
            };
            if let Some((n, _)) =
                parse_number(condition_tokens.get(count_start..).unwrap_or_default())
            {
                crate::static_abilities::ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-spell cost condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    if parse_self_free_cast_alternative_cost_line(&tail_tokens).is_some() {
        let method = AlternativeCastingMethod::alternative_cost_with_condition(
            "Parsed alternative cost",
            None,
            Vec::new(),
            condition,
        );
        if let Some(trap_condition) = method
            .cast_condition()
            .and_then(trap_condition_from_this_spell_cost_condition)
            && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
        {
            return Ok(Some(AlternativeCastingMethod::trap(
                "Trap",
                cost,
                trap_condition,
            )));
        }
        return Ok(Some(method));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)? else {
        return Ok(None);
    };
    if lex_line(line, 0)
        .ok()
        .is_some_and(|tokens| token_slice_starts_with(&tokens, &["freerunning"]))
        && let Some(cost) = method.mana_cost().cloned()
    {
        return Ok(Some(
            AlternativeCastingMethod::alternative_cost_with_condition(
                "Freerunning",
                Some(cost),
                method.non_mana_costs(),
                condition,
            ),
        ));
    }
    let method = method.with_cast_condition(condition);
    if let Some(trap_condition) = method
        .cast_condition()
        .and_then(trap_condition_from_this_spell_cost_condition)
        && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
    {
        return Ok(Some(AlternativeCastingMethod::trap(
            "Trap",
            cost,
            trap_condition,
        )));
    }
    Ok(Some(method))
}

pub(crate) fn parse_if_conditional_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_if_conditional_alternative_cost_line(tokens, line)
}
