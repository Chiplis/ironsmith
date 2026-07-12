#[cfg(test)]
use crate::PtValue;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::TextSpan;
use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CHOSEN_OBJECTS_TAG, CardTextError, IT_TAG, KeywordAction,
    ParsedAbility, PlayerAst, ReferenceImports, TargetAst,
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
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use crate::{ChoiceCount, PowerToughness, TagKey};

use super::activation_and_restrictions::activated_line_core::parse_activation_cost;
use super::clause_support::parse_effect_sentences_lexed;
use super::effect_sentences::find_verb;
use super::grammar::abilities::{
    FlashbackCostClause, parse_flashback_cost_clause_tokens,
    parse_flashback_keyword_line_spec_lexed,
};
pub(crate) use super::grammar::filters::{
    intern_counter_name, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_counter_type_words, parse_filter_counter_constraint_words,
};
use super::grammar::leaf;
use super::grammar::primitives::token_slice_span;
use super::grammar::shared_util::additional_cost_choices;
use super::grammar::shared_util::alternative_cost_lines;
use super::grammar::shared_util::cast_restriction_lines;
use super::grammar::shared_util::count_shapes;
use super::grammar::shared_util::header_shapes;
use super::grammar::shared_util::keyword_cost_lines;
use super::grammar::shared_util::keyword_line_facts::{self, MadnessCostFact, NamedCostKeyword};
use super::grammar::shared_util::reference_shapes;
pub(crate) use super::grammar::shared_util::reference_shapes::{
    FilterKeywordConstraint, SubjectAst,
};
use super::grammar::shared_util::target_semantics;
use super::grammar::shared_util::token_facts;
use super::grammar::shared_util::value_expr;
use super::grammar::shared_util::value_shapes;
use super::grammar::targets::{
    EnchantedObjectTargetKind, TargetControllerSetConstraint, TargetPreparationFacts,
    TargetUnionShape, parse_chosen_object_target, parse_enchanted_object_target_kind,
    parse_referenced_target_prefix, parse_target_controller_set_suffix, parse_target_envelope,
    parse_target_for_each_suffix, parse_target_preparation_facts, parse_target_union_shape,
};
use super::keyword_static::parse_this_spell_cost_condition;
use super::lexer::{OwnedLexToken, TokenKind, lex_line, render_token_slice};
use super::object_filters::{parse_object_filter, parse_object_filter_words};
use super::token_primitives as shared_tokens;
use std::cell::RefCell;
use std::collections::HashMap;

const SACRIFICE_COST_TAG_PREFIX: &str = "sacrifice_cost_";
const EXILE_COST_TAG_PREFIX: &str = "exile_cost_";
const UNATTACH_COST_TAG_PREFIX: &str = "unattach_cost_";
type SourceReferenceAlias = leaf::LeafSourceReferenceAlias;

#[derive(Clone, Default)]
struct SourceReferenceContext {
    source_name: String,
    aliases: Vec<SourceReferenceAlias>,
    preferred_self_surface: Option<SourceReferenceSurface>,
    surfaces_by_span: HashMap<TextSpan, SourceReferenceSurface>,
}

thread_local! {
    static SOURCE_REFERENCE_CONTEXT: RefCell<SourceReferenceContext> =
        RefCell::new(SourceReferenceContext::default());
}

#[allow(dead_code)]
pub(crate) fn with_source_reference_context<T>(card_name: &str, f: impl FnOnce() -> T) -> T {
    with_source_reference_context_aliases(card_name, Vec::new(), f)
}

pub(crate) fn with_card_source_reference_context<T>(
    card_name: &str,
    card_types: &[CardType],
    subtypes: &[Subtype],
    f: impl FnOnce() -> T,
) -> T {
    with_source_reference_context_aliases(
        card_name,
        source_reference_aliases_for_card_identity(card_types, subtypes),
        f,
    )
}

fn with_source_reference_context_aliases<T>(
    card_name: &str,
    extra_aliases: Vec<SourceReferenceAlias>,
    f: impl FnOnce() -> T,
) -> T {
    let preferred_self_surface = extra_aliases.first().map(|alias| alias.surface.clone());
    let mut aliases = source_reference_aliases_for_name(card_name);
    for alias in extra_aliases {
        push_source_reference_alias_words(&mut aliases, alias.words, alias.surface);
    }
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.words.len()));
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let previous = context.replace(SourceReferenceContext {
            source_name: card_name.trim().to_string(),
            aliases,
            preferred_self_surface,
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

pub(crate) fn preferred_source_reference_self_surface() -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| context.borrow().preferred_self_surface.clone())
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
    leaf::parse_leaf_source_reference_aliases_for_name(name)
}

fn source_reference_aliases_for_card_identity(
    card_types: &[CardType],
    subtypes: &[Subtype],
) -> Vec<SourceReferenceAlias> {
    let mut aliases = Vec::new();
    for card_type in card_types {
        if let Some(type_name) = source_reference_self_card_type_name(*card_type) {
            push_this_source_reference_alias(&mut aliases, type_name);
        }
    }
    for subtype in subtypes {
        if is_permanent_source_reference_subtype(*subtype) {
            push_this_source_reference_alias(&mut aliases, &subtype.display_name());
        }
    }
    aliases
}

fn source_reference_self_card_type_name(card_type: CardType) -> Option<&'static str> {
    match card_type {
        CardType::Artifact
        | CardType::Battle
        | CardType::Creature
        | CardType::Enchantment
        | CardType::Land
        | CardType::Planeswalker => Some(card_type.name()),
        CardType::Instant | CardType::Kindred | CardType::Sorcery => None,
    }
}

fn is_permanent_source_reference_subtype(subtype: Subtype) -> bool {
    subtype.is_land_subtype()
        || subtype.is_creature_type()
        || subtype.is_artifact_subtype()
        || subtype.is_enchantment_subtype()
        || subtype.is_planeswalker_subtype()
        || subtype.is_battle_subtype()
}

fn push_this_source_reference_alias(aliases: &mut Vec<SourceReferenceAlias>, permanent_type: &str) {
    let Some(surface) = leaf::parse_leaf_this_source_reference_surface(permanent_type) else {
        return;
    };
    let SourceReferenceSurface::ThisPermanentType(surface_text) = &surface else {
        return;
    };
    let surface_text = surface_text.clone();
    leaf::push_leaf_source_reference_alias(aliases, &surface_text, surface);
}

fn push_source_reference_alias_words(
    aliases: &mut Vec<SourceReferenceAlias>,
    words: Vec<String>,
    surface: SourceReferenceSurface,
) {
    leaf::push_leaf_source_reference_alias_words(aliases, words, surface);
}

pub(crate) fn source_reference_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        leaf::parse_leaf_source_reference_alias_words(&context.borrow().aliases, words)
    })
}

pub(crate) fn source_reference_surface_for_possessive_words(
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        leaf::parse_leaf_source_reference_possessive_alias_words(&context.borrow().aliases, words)
    })
}

pub(crate) fn source_choose_spec_for_surface(surface: SourceReferenceSurface) -> ChooseSpec {
    ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface))
}

pub(crate) fn this_source_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    leaf::parse_leaf_this_source_reference_words(words)
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

pub(crate) fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    count_shapes::parse_for_each_count_value_words(words)
}

pub(crate) fn is_article(word: &str) -> bool {
    leaf::parse_leaf_article_complete(word).is_ok()
}

pub(crate) fn strip_leading_word_refs_any<'slice, 'word>(
    words: &'slice [&'word str],
    leading_words: &[&str],
) -> &'slice [&'word str] {
    let fact = token_facts::strip_leading_selected_word_refs_lexical(words, leading_words);
    &words[fact.consumed_words..]
}

pub(crate) fn strip_leading_article_word_refs<'slice, 'word>(
    words: &'slice [&'word str],
) -> &'slice [&'word str] {
    leaf::parse_leaf_leading_articles_words(words).rest
}

pub(crate) fn strip_leading_article_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    leaf::parse_leaf_leading_articles_tokens(tokens).rest
}

pub(crate) fn strip_leading_token_words_any<'a>(
    tokens: &'a [OwnedLexToken],
    leading_words: &[&str],
) -> &'a [OwnedLexToken] {
    leaf::parse_leaf_leading_selected_tokens(tokens, leading_words).rest
}

pub(crate) fn parse_choice_count_word_prefix(words: &[&str]) -> Option<(ChoiceCount, usize)> {
    let parsed = leaf::parse_leaf_choice_count_prefix_words(words)?;
    Some((parsed.count, parsed.consumed))
}

pub(crate) fn strip_leading_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    strip_leading_article_tokens(tokens).to_vec()
}

pub(crate) fn word_refs_at_is_article(words: &[&str], idx: usize) -> bool {
    words.get(idx).is_some_and(|word| is_article(word))
}

pub(crate) fn non_article_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    token_facts::non_article_word_refs(words)
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
    token_facts::non_article_token_word_refs(tokens)
}

pub(crate) fn strip_possessive_suffix(word: &str) -> &str {
    leaf::strip_leaf_source_possessive_suffix(word)
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

const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper_";
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

pub(crate) fn classify_instead_followup_tokens(
    tokens: &[OwnedLexToken],
) -> crate::cards::builders::InsteadSemantics {
    super::grammar::effects::classify_instead_followup_semantics_tokens(tokens)
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
    tag.as_str().get(..prefix.len()) == Some(prefix)
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
    token_facts::parse_activation_cost_start_tokens(tokens)
        .is_some_and(|fact| fact.token_index == 0)
}

pub(crate) fn find_activation_cost_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    token_facts::parse_activation_cost_start_tokens(tokens).map(|fact| fact.token_index)
}

pub(crate) fn contains_source_from_your_graveyard_phrase(words: &[&str]) -> bool {
    reference_shapes::contains_source_from_your_graveyard(words)
}

pub(crate) fn contains_source_from_your_hand_phrase(words: &[&str]) -> bool {
    reference_shapes::contains_source_from_your_hand(words)
}

pub(crate) fn contains_from_command_zone_phrase(words: &[&str]) -> bool {
    reference_shapes::contains_from_command_zone(words)
}

pub(crate) fn contains_discard_source_phrase(words: &[&str]) -> bool {
    reference_shapes::contains_discard_source(words)
}

pub(crate) fn is_basic_color_word(word: &str) -> bool {
    token_facts::parse_basic_color_word(word).is_some()
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
    token_facts::parse_cost_segments_tokens(tokens)
        .segments
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
}

pub(crate) fn parse_next_end_step_token_delay_flags(
    tail_words: &[&str],
) -> (bool, bool, PlayerFilter) {
    let Some(facts) = super::grammar::effects::parse_next_end_step_delay_words(tail_words) else {
        return (false, false, PlayerFilter::Any);
    };
    (
        facts.sacrifice_reference,
        facts.exile_reference,
        facts.player,
    )
}

pub(crate) fn token_boundary_for_word(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
    token_facts::token_boundary_for_word(tokens, word_index)
}

pub(crate) fn remove_first_may_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some(fact) = token_facts::parse_first_may_word_token(tokens) else {
        return tokens.to_vec();
    };
    tokens[..fact.token_index]
        .iter()
        .chain(tokens[fact.token_index + 1..].iter())
        .cloned()
        .collect()
}

pub(crate) fn remove_through_first_may_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some(fact) = token_facts::parse_first_may_word_token(tokens) else {
        return Vec::new();
    };
    tokens[fact.token_index + 1..].to_vec()
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

pub(crate) fn contains_until_end_of_turn(words: &[&str]) -> bool {
    leaf::find_leaf_canonical_until_end_of_turn_words(words).is_some()
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
    leaf::parse_leaf_card_type_complete(word).ok()
}

pub(crate) fn parse_supertype_word(word: &str) -> Option<Supertype> {
    leaf::parse_leaf_supertype_complete(word).ok()
}

pub(crate) fn parse_subtype_word(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_subtype_complete(word).ok()
}

pub(crate) fn parse_mana_symbol_word_flexible(word: &str) -> Option<ManaSymbol> {
    leaf::parse_leaf_spelled_mana_word_complete(word).ok()
}

pub(crate) fn parse_color(word: &str) -> Option<crate::color::ColorSet> {
    leaf::parse_leaf_color_complete(word).ok()
}

pub(crate) fn parse_non_type(word: &str) -> Option<CardType> {
    leaf::parse_leaf_non_card_type_complete(word).ok()
}

pub(crate) fn parse_non_supertype(word: &str) -> Option<Supertype> {
    leaf::parse_leaf_non_supertype_complete(word).ok()
}

pub(crate) fn parse_non_color(word: &str) -> Option<crate::color::ColorSet> {
    leaf::parse_leaf_non_color_complete(word).ok()
}

pub(crate) fn parse_non_subtype(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_non_subtype_complete(word).ok()
}

pub(crate) fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_subtype_flexible_complete(word).ok()
}

pub(crate) fn is_source_reference_words(words: &[&str]) -> bool {
    is_this_source_reference_words(words) || source_reference_surface_for_words(words).is_some()
}

fn is_this_source_reference_words(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
}

pub(crate) fn is_demonstrative_object_head(word: &str) -> bool {
    leaf::parse_leaf_demonstrative_object_head_complete(word).is_ok()
}

pub(crate) fn is_outlaw_word(word: &str) -> bool {
    token_facts::parse_outlaw_word(word) == Some(token_facts::OutlawWord::Outlaw)
}

pub(crate) fn is_non_outlaw_word(word: &str) -> bool {
    token_facts::parse_outlaw_word(word) == Some(token_facts::OutlawWord::NonOutlaw)
}

pub(crate) fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
    for subtype in [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ] {
        if !out.contains(&subtype) {
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
    leaf::parse_leaf_zone_complete(word).ok()
}

pub(crate) fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    let parsed = leaf::parse_leaf_alternative_cast_prefix_words(words)?;
    Some((parsed.kind, parsed.consumed))
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    leaf::parse_leaf_unsigned_pt_complete(word).ok()
}

pub(crate) fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    reference_shapes::parse_filter_keyword_constraint_words(words)
}

pub(crate) fn word_is_cycling_keyword_marker(word: &str) -> bool {
    reference_shapes::cycling_keyword_root(word).is_some()
}

pub(crate) fn cycling_keyword_root(word: &str) -> Option<&str> {
    reference_shapes::cycling_keyword_root(word)
}

pub(crate) fn apply_filter_keyword_constraint(
    filter: &mut ObjectFilter,
    constraint: FilterKeywordConstraint,
    excluded: bool,
) {
    match constraint {
        FilterKeywordConstraint::Static(ability_id) => {
            if excluded {
                if !filter.excluded_static_abilities.contains(&ability_id) {
                    filter.excluded_static_abilities.push(ability_id);
                }
            } else if !filter.static_abilities.contains(&ability_id) {
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
    let spec = parse_flashback_keyword_line_spec_lexed(tokens)?;
    let mut text = format!("Flashback {}", spec.cost.to_oracle());
    let tail = words(spec.tail_tokens);
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

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_tokens::parse_scryfall_mana_cost(raw)
}

pub(crate) fn parse_number_or_x_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    leaf::parse_leaf_number_or_x_prefix_tokens(tokens)?.into_value()
}

pub(crate) fn parse_number_or_x_value_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_number_or_x_value(tokens)
}

pub(crate) fn parse_number_word_i32(word: &str) -> Option<i32> {
    leaf::parse_number_i32_complete(word).ok()
}

pub(crate) fn parse_number_word_u32(word: &str) -> Option<u32> {
    parse_number_word_i32(word).and_then(|value| value.try_into().ok())
}

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    value_expr::parse_value_expr_words(words)
}

pub(crate) fn parse_value_expr(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    value_expr::parse_value_expr_tokens(tokens)
}

pub(crate) fn parse_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_value_expr(tokens)
}

pub(crate) fn parse_subject(tokens: &[OwnedLexToken]) -> SubjectAst {
    reference_shapes::parse_subject_tokens(tokens)
}

pub(crate) fn span_from_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    token_slice_span(tokens)
}

pub(crate) fn parse_number(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    leaf::parse_leaf_number_prefix_tokens(tokens)?.into_fixed()
}

pub(crate) fn parse_quantity_comparison_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    let parsed = value_shapes::parse_quantity_comparison_prefix_tokens(
        tokens,
        allow_default_one,
        article_implies_min_one,
    )
    .ok_or_else(|| CardTextError::ParseError(format!("missing quantity in {error_context}")))?;
    Ok((parsed.comparison, parsed.consumed_tokens))
}

pub(crate) fn parse_quantity_comparison_prefix_words(
    words: &[&str],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    let parsed = value_shapes::parse_quantity_comparison_prefix_words(
        words,
        allow_default_one,
        article_implies_min_one,
    )
    .ok_or_else(|| CardTextError::ParseError(format!("missing quantity in {error_context}")))?;
    Ok((parsed.comparison, parsed.consumed_words))
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

pub(crate) fn parse_greater_than_or_equal_quantity_prefix_words(
    words: &[&str],
    allow_default_one: bool,
    article_implies_min_one: bool,
    error_context: &str,
) -> Result<Option<(u32, usize)>, CardTextError> {
    let (comparison, used) = parse_quantity_comparison_prefix_words(
        words,
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
    let (count, rest) = super::grammar::primitives::parse_prefix(
        tokens,
        super::grammar::leaf::parse_leaf_target_count_range_prefix_lexed,
    )?;
    Some((count, tokens.len().checked_sub(rest.len())?))
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
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    Some((parsed.count, parsed.consumed))
}

pub(crate) fn parse_choice_count_before_target_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let fact = token_facts::parse_choice_count_before_target_tokens(tokens)?;
    Some((fact.count, fact.consumed_tokens))
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

        let this_creature = lex_line("this creature", 0).unwrap();
        assert!(matches!(
            parse_target_phrase(&this_creature).expect("this creature should parse as the source"),
            TargetAst::Source(_)
        ));
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
    fn parse_target_phrase_recognizes_nonattacking_nonblocking_target_creature() {
        let tokens = lex_line("target nonattacking, nonblocking creature", 0).unwrap();
        let target = parse_target_phrase(&tokens)
            .expect("nonattacking nonblocking target creature should parse");

        let TargetAst::Object(filter, target_span, _) = target else {
            panic!("expected target object, got {target:?}");
        };
        assert!(target_span.is_some(), "expected explicit target span");
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(
            filter.nonattacking,
            "expected nonattacking filter: {filter:?}"
        );
        assert!(
            filter.nonblocking,
            "expected nonblocking filter: {filter:?}"
        );
    }

    #[test]
    fn parse_target_phrase_exiled_card_target_is_not_source_linked() {
        let tokens = lex_line("target face-up exiled card", 0).unwrap();
        let target = parse_target_phrase(&tokens).expect("target face-up exiled card should parse");

        let TargetAst::Object(filter, target_span, _) = target else {
            panic!("expected target object, got {target:?}");
        };
        assert!(target_span.is_some(), "expected explicit target span");
        assert_eq!(filter.zone, Some(Zone::Exile));
        assert_eq!(filter.face_down, Some(false));
        assert!(
            !filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            }),
            "explicit exiled-card target should not be source-linked: {filter:?}"
        );
    }

    #[test]
    fn parse_target_phrase_this_card_from_exile_keeps_source_filter() {
        let tokens = lex_line("this card from exile", 0).unwrap();
        let target = parse_target_phrase(&tokens).expect("this card from exile should parse");

        let TargetAst::Object(filter, target_span, _) = target else {
            panic!("expected source object filter, got {target:?}");
        };
        assert!(target_span.is_none(), "expected no explicit target span");
        assert_eq!(filter.zone, Some(Zone::Exile));
        assert!(filter.source, "expected source filter: {filter:?}");
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

    #[test]
    fn aggregate_scope_values_use_captured_metric_and_scope_shape() {
        let each_words = [
            "for",
            "each",
            "color",
            "among",
            "the",
            "creatures",
            "you",
            "control",
        ];
        let (each_value, each_used) =
            parse_for_each_count_value_words(&each_words).expect("aggregate count should parse");
        assert_eq!(each_used, each_words.len());
        let Value::ColorsAmong(each_filter) = each_value else {
            panic!("expected colors-among value, got {each_value:?}");
        };
        assert_eq!(each_filter.card_types, vec![CardType::Creature]);
        assert_eq!(each_filter.controller, Some(PlayerFilter::You));

        let value_words = [
            "different",
            "powers",
            "among",
            "creatures",
            "you",
            "control",
        ];
        let (value, used) =
            parse_value_expr_words(&value_words).expect("aggregate value should parse");
        assert_eq!(used, value_words.len());
        let Value::DistinctPowers(filter) = value else {
            panic!("expected distinct-powers value, got {value:?}");
        };
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn spell_cast_this_turn_count_uses_captured_filter_and_suffix() {
        let words = ["other", "creature", "spells", "you", "cast", "this", "turn"];
        let (value, used) =
            parse_value_expr_words(&words).expect("spell-cast count value should parse");
        assert_eq!(used, words.len());
        let Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } = value
        else {
            panic!("expected spell-cast matching value, got {value:?}");
        };
        assert_eq!(player, PlayerFilter::You);
        assert!(exclude_source);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
    }

    #[test]
    fn parse_for_each_count_value_words_handles_fade_counters_on_source() {
        let words = ["for", "each", "fade", "counter", "on", "this", "artifact"];

        let (value, used_words) =
            parse_for_each_count_value_words(&words).expect("fade counter count should parse");

        assert_eq!(used_words, words.len());
        let Value::CountersOn(spec, Some(CounterType::Fade)) = value else {
            panic!("expected source fade counter count, got {value:?}");
        };
        assert_eq!(describe_choose_spec_for_test(&spec), "this artifact");
    }

    #[test]
    fn parse_target_phrase_handles_dynamic_count_for_each_source_counter() {
        let tokens = lex_line(
            "an untapped artifact, creature, or land they control for each fade counter on this artifact",
            0,
        )
        .unwrap();

        let target = parse_target_phrase(&tokens).expect("dynamic counted target should parse");

        let TargetAst::WithCountValue(inner, count, value) = target else {
            panic!("expected dynamic-count target, got {target:?}");
        };
        assert!(count.is_dynamic_x());
        let Value::CountersOn(spec, Some(CounterType::Fade)) = value else {
            panic!("expected source fade counter count, got {value:?}");
        };
        assert_eq!(describe_choose_spec_for_test(&spec), "this artifact");
        let TargetAst::Object(filter, _, _) = inner.as_ref() else {
            panic!("expected object target, got {inner:?}");
        };
        assert!(filter.untapped, "expected untapped filter, got {filter:?}");
        assert!(
            filter.controller.is_some(),
            "expected controller filter, got {filter:?}"
        );
        assert!(filter.card_types.contains(&CardType::Artifact));
        assert!(filter.card_types.contains(&CardType::Creature));
        assert!(filter.card_types.contains(&CardType::Land));
    }

    #[test]
    fn typed_keyword_line_facts_feed_semantic_adapters() {
        let level = lex_line("Level up {2}{U}", 0).unwrap();
        assert!(parse_level_up_line(&level).unwrap().is_some());

        let madness = lex_line("Madness—Pay three {B}.", 0).unwrap();
        assert!(parse_madness_line(&madness).unwrap().is_some());

        let bargain = lex_line("Bargain", 0).unwrap();
        assert!(parse_bargain_line(&bargain).unwrap().is_some());

        let replicate = lex_line("Replicate—{1}{U}.", 0).unwrap();
        assert!(parse_replicate_line(&replicate).unwrap().is_some());

        let escalate = lex_line("Escalate {1}{R}", 0).unwrap();
        assert!(parse_escalate_line_lexed(&escalate).unwrap().is_some());

        let evoke = lex_line("Evoke {2}{B}", 0).unwrap();
        assert!(parse_evoke_line_lexed(&evoke).unwrap().is_some());

        let prowl = lex_line("Prowl {1}{B}", 0).unwrap();
        assert!(parse_prowl_line_lexed(&prowl).unwrap().is_some());

        let eternalize = lex_line("Eternalize {4}{U}{U}", 0).unwrap();
        assert!(parse_eternalize_line_lexed(&eternalize).unwrap().is_some());

        let epic = lex_line("Epic", 0).unwrap();
        assert!(parse_epic_line_lexed(&epic));

        let retrace = lex_line("Retrace", 0).unwrap();
        assert!(parse_retrace_line(&retrace).unwrap().is_some());

        let harmonize = lex_line("Harmonize {2}{G}", 0).unwrap();
        assert!(parse_harmonize_line(&harmonize).unwrap().is_some());

        let warp = lex_line("Warp {1}{R}", 0).unwrap();
        assert!(parse_warp_line(&warp).unwrap().is_some());

        let reinforce = lex_line("Reinforce 2 {1}{G}", 0).unwrap();
        assert!(parse_reinforce_line(&reinforce).unwrap().is_some());
    }

    fn describe_choose_spec_for_test(spec: &ChooseSpec) -> String {
        match spec {
            ChooseSpec::SurfaceHinted { hints, .. } => hints
                .iter()
                .find_map(|hint| match hint {
                    ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::FullName(
                        text,
                    ))
                    | ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ShortName(
                        text,
                    ))
                    | ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType(text),
                    ) => Some(text.clone()),
                })
                .unwrap_or_else(|| format!("{spec:?}")),
            _ => format!("{spec:?}"),
        }
    }
}

pub(crate) fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

pub(crate) fn is_source_from_your_graveyard_words(words: &[&str]) -> bool {
    reference_shapes::is_source_from_your_graveyard(words)
}

pub(crate) fn parse_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let envelope = parse_target_envelope(tokens);
    if let Some(count) = envelope.counted_any_target {
        return Ok(TargetAst::WithCount(
            Box::new(TargetAst::AnyTarget(span_from_tokens(tokens))),
            count,
        ));
    }

    match parse_target_phrase_inner(tokens) {
        Ok(target) => Ok(target),
        Err(err) => {
            for candidate in envelope.recovery_candidates {
                if let Ok(target) = parse_target_phrase_inner(candidate.tokens) {
                    return Ok(target);
                }
            }
            Err(err)
        }
    }
}

fn parse_target_phrase_inner(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    target_semantics::parse_target_phrase_inner(tokens)
}

pub(crate) fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, String)> {
    let parsed = header_shapes::parse_saga_chapter_header(line)?;
    Some((parsed.chapters, parsed.body))
}

pub(crate) fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let parsed = header_shapes::parse_level_header(line)?;
    Some((parsed.minimum, parsed.maximum))
}

pub(crate) fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    leaf::parse_leaf_power_toughness_complete(raw).ok()
}

pub(crate) fn parse_level_up_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(fact) = keyword_line_facts::parse_level_up_line_tokens(tokens) else {
        return Ok(None);
    };
    let mana_cost = fact
        .mana_cost
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

pub(crate) fn parse_self_free_cast_alternative_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    alternative_cost_lines::parse_self_free_cast(tokens)
}

pub(crate) fn parse_self_free_cast_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_self_free_cast_alternative_cost_line(tokens)
}

pub(crate) fn parse_flash_with_additional_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    alternative_cost_lines::parse_flash_with_additional_cost(tokens)
}

pub(crate) fn parse_flash_with_additional_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_flash_with_additional_cost_line(tokens)
}

pub(crate) fn mana_pips_from_token(token: &OwnedLexToken) -> Option<Vec<ManaSymbol>> {
    leaf::parse_leaf_surface_mana_pip_token(token).map(leaf::LeafManaPipToken::into_pip)
}

pub(crate) fn leading_mana_cost_from_tokens(tokens: &[OwnedLexToken]) -> Option<(ManaCost, usize)> {
    let prefix = leaf::parse_leaf_mana_cost_prefix_tokens(tokens)?;
    Some((prefix.cost, prefix.consumed))
}

pub(crate) fn parse_madness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(fact) = keyword_line_facts::parse_madness_line_tokens(tokens) else {
        return Ok(None);
    };
    let mana_cost = match fact.cost {
        MadnessCostFact::RepeatedMana(mana_cost) => mana_cost,
        MadnessCostFact::ActivationTokens(cost_tokens) => {
            if cost_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "madness keyword missing mana cost".to_string(),
                ));
            }
            parse_activation_cost(cost_tokens)?
                .mana_cost()
                .cloned()
                .ok_or_else(|| {
                    CardTextError::ParseError("madness keyword missing mana symbols".to_string())
                })?
        }
    };

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
    keyword_cost_lines::parse_buyback(tokens)
}

pub(crate) fn parse_buyback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_buyback_line(tokens)
}

pub(crate) fn parse_bargain_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    if keyword_line_facts::parse_bargain_line_tokens(tokens).is_none() {
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
    keyword_cost_lines::parse_optional_cost(tokens, keyword, constructor)
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
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Replicate)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "replicate keyword missing cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(fact.cost_tokens)?;
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

pub(crate) fn parse_escalate_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(TotalCost, String)>, CardTextError> {
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Escalate)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Ok(None);
    }
    let total_cost = parse_activation_cost(fact.cost_tokens)?;
    let display = render_token_slice(fact.cost_tokens).trim().to_string();
    Ok(Some((total_cost, display)))
}

pub(crate) fn parse_evoke_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Evoke)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Ok(None);
    }
    let total_cost = parse_activation_cost(fact.cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Evoke",
        total_cost,
        condition: None,
        prototype_power_toughness: None,
    }))
}

pub(crate) fn parse_prowl_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Prowl)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Ok(None);
    }
    let total_cost = parse_activation_cost(fact.cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Prowl",
        total_cost,
        condition: Some(
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue,
            ),
        ),
        prototype_power_toughness: None,
    }))
}

pub(crate) fn parse_eternalize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ManaCost>, CardTextError> {
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Eternalize)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Ok(None);
    }
    let (mana_cost, consumed) =
        leading_mana_cost_from_tokens(fact.cost_tokens).ok_or_else(|| {
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
    keyword_line_facts::parse_epic_line_tokens(tokens).is_some()
}

pub(crate) fn parse_morph_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    keyword_cost_lines::parse_morph(tokens)
}

pub(crate) fn parse_morph_keyword_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_morph_keyword_line(tokens)
}

pub(crate) fn parse_escape_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    keyword_cost_lines::parse_escape(tokens)
}

pub(crate) fn parse_escape_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_escape_line(tokens)
}

pub(crate) fn parse_flashback_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let cost_tokens = match parse_flashback_cost_clause_tokens(tokens) {
        None => return Ok(None),
        Some(FlashbackCostClause::Missing) => {
            return Err(CardTextError::ParseError(
                "flashback keyword missing mana cost".to_string(),
            ));
        }
        Some(FlashbackCostClause::UnsupportedCostsClause(cost_tokens)) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported activation cost segment (clause: '{}')",
                words(cost_tokens).join(" ")
            )));
        }
        Some(FlashbackCostClause::Cost(cost_tokens)) => cost_tokens,
    };

    let total_cost = match parse_activation_cost(cost_tokens) {
        Ok(total_cost) => total_cost,
        Err(_) => {
            crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
                cost_tokens,
            )?
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported flashback cost (clause: '{}')",
                    words(cost_tokens).join(" ")
                ))
            })?
        }
    };

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
    if keyword_line_facts::parse_retrace_line_tokens(tokens).is_none() {
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
    Ok(keyword_cost_lines::parse_jump_start(tokens))
}

pub(crate) fn parse_jump_start_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_jump_start_line(tokens)
}

pub(crate) fn parse_harmonize_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(fact) = keyword_line_facts::parse_harmonize_line_tokens(tokens) else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "harmonize keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(fact.cost_tokens)?;
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
    let Some(fact) = keyword_line_facts::parse_warp_line_tokens(tokens) else {
        return Ok(None);
    };
    let (cost, _) = leading_mana_cost_from_tokens(fact.cost_tokens)
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
    keyword_cost_lines::parse_bestow(tokens)
}

pub(crate) fn parse_bestow_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_bestow_line(tokens)
}

pub(crate) fn parse_blitz_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    keyword_cost_lines::parse_blitz(tokens)
}

pub(crate) fn parse_blitz_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_blitz_line(tokens)
}

pub(crate) fn parse_transmute_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    keyword_cost_lines::parse_transmute(tokens)
}

pub(crate) fn parse_transmute_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_transmute_line(tokens)
}

pub(crate) fn parse_reinforce_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(fact) = keyword_line_facts::parse_reinforce_line_tokens(tokens) else {
        return Ok(None);
    };
    let words_all = words(tokens);
    let Some(amount_value) = fact.amount.and_then(leaf::LeafNumber::into_value) else {
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

    if fact.cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let Some((base_mana_cost, _consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(fact.cost_tokens)
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
    cast_restriction_lines::parse_cast_this_spell_only(tokens)
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
    alternative_cost_lines::parse_you_may_rather_than_spell_cost(tokens, line)
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_you_may_rather_than_spell_cost_line(tokens, line)
}

pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    additional_cost_choices::parse_additional_cost_choices(tokens)
}

pub(crate) fn parse_additional_cost_choice_options_lexed(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<Vec<AdditionalCostChoiceOptionAst<crate::runtime_backend::ast::EffectAst>>>,
    CardTextError,
> {
    parse_additional_cost_choice_options(tokens)
}

pub(crate) fn parse_if_conditional_alternative_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    alternative_cost_lines::parse_if_conditional_alternative_cost(tokens, line)
}

pub(crate) fn parse_if_conditional_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_if_conditional_alternative_cost_line(tokens, line)
}
