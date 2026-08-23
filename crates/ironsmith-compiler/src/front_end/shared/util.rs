#[cfg(test)]
use crate::PtValue;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::TextSpan;
#[cfg(test)]
use crate::cards::builders::PlayerAst;
use crate::cards::builders::{
    ADDITIONAL_COST_OBJECT_TAG, AdditionalCostChoiceOptionAst, CardTextError, KeywordAction,
    ParsedAbility, ReferenceImports, TargetAst,
};
use crate::cost::OptionalCost;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, Value};
use crate::filter::AlternativeCastKind;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
#[cfg(test)]
use crate::target::TaggedOpbjectRelation;
use crate::target::{
    ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter, SacrificedObjectKind,
    SourceReferenceSurface,
};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use crate::{ChoiceCount, PowerToughness, TagKey};
use ironsmith_core::CostComponent as _;

use super::activation_and_restrictions::activated_line_core::parse_activation_cost;
use super::grammar::abilities::{
    FlashbackCostClause, parse_flashback_cost_clause_tokens,
    parse_flashback_keyword_line_spec_lexed,
};
pub use super::grammar::filters::{
    intern_counter_name, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_filter_counter_constraint_words,
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
pub use super::grammar::shared_util::reference_shapes::{FilterKeywordConstraint, SubjectAst};
use super::grammar::shared_util::target_semantics;
use super::grammar::shared_util::token_facts;
use super::grammar::shared_util::value_expr;
use super::grammar::shared_util::value_shapes;
use super::grammar::targets::parse_target_envelope;
#[cfg(test)]
use super::lexer::lex_line;
use super::lexer::{OwnedLexToken, TokenKind, render_token_slice};
use super::token_primitives as shared_tokens;
const SACRIFICE_COST_TAG_PREFIX: &str = "sacrifice_cost_";
const EXILE_COST_TAG_PREFIX: &str = "exile_cost_";
const UNATTACH_COST_TAG_PREFIX: &str = "unattach_cost_";
const TAP_COST_TAG_PREFIX: &str = "tap_cost_";
const RETURN_COST_TAG_PREFIX: &str = "return_cost_";
const DISCARD_COST_TAG_PREFIX: &str = "discard_cost_";
const DISCARDED_COST_TAG: &str = "discarded_cost";
#[cfg(test)]
type SourceReferenceAlias = leaf::LeafSourceReferenceAlias;
/// Run a nested parse with only the card's proper-name aliases installed.
///
/// Most full-card parsing should use `with_card_source_reference_context`,
/// which also makes `this creature`, `this enchantment`, and similar type
/// surfaces the preferred self-reference. Quoted abilities granted to an
/// attached object are different: an explicit proper name still denotes the
/// granting card, while a typed `this ...` reference denotes the object that
/// receives the ability. The attached-grant parser uses this name-only context
/// to preserve that authored distinction.
pub fn with_source_reference_context<T>(_card_name: &str, f: impl FnOnce() -> T) -> T {
    f()
}

pub fn with_card_source_reference_context<T>(
    _card_name: &str,
    _card_types: &[CardType],
    _subtypes: &[Subtype],
    f: impl FnOnce() -> T,
) -> T {
    f()
}

pub fn with_token_source_reference_context<T>(
    _token_name: &str,
    _card_types: &[CardType],
    _subtypes: &[Subtype],
    f: impl FnOnce() -> T,
) -> T {
    f()
}

pub fn current_source_reference_name() -> Option<String> {
    None
}

pub fn preferred_source_reference_self_surface() -> Option<SourceReferenceSurface> {
    None
}

pub fn source_reference_surface_for_span(
    _span: Option<TextSpan>,
) -> Option<SourceReferenceSurface> {
    None
}

pub fn record_source_reference_surface(_span: Option<TextSpan>, _surface: SourceReferenceSurface) {}

pub fn sacrificed_object_kind_for_span(_span: Option<TextSpan>) -> Option<SacrificedObjectKind> {
    None
}

pub fn record_sacrificed_object_kind(_span: Option<TextSpan>, _kind: SacrificedObjectKind) {}

#[cfg(test)]
fn source_reference_aliases_for_name(name: &str) -> Vec<SourceReferenceAlias> {
    leaf::parse_leaf_source_reference_aliases_for_name(name)
}

#[cfg(test)]
fn canonical_source_reference_surface(
    aliases: &[SourceReferenceAlias],
    surface: SourceReferenceSurface,
) -> SourceReferenceSurface {
    let surface_text = match &surface {
        SourceReferenceSurface::FullName(text) | SourceReferenceSurface::ShortName(text) => text,
        SourceReferenceSurface::ThisPermanentType(_) => return surface,
    };
    aliases
        .iter()
        .find_map(|alias| match &alias.surface {
            SourceReferenceSurface::FullName(alias_text)
            | SourceReferenceSurface::ShortName(alias_text)
                if alias_text.eq_ignore_ascii_case(surface_text) =>
            {
                Some(alias.surface.clone())
            }
            _ => None,
        })
        .unwrap_or(surface)
}

pub fn source_reference_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    leaf::parse_leaf_this_source_reference_words(words)
}

pub fn source_reference_surface_for_possessive_words(
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    leaf::parse_leaf_this_source_reference_words(words)
}

pub fn source_choose_spec_for_surface(surface: SourceReferenceSurface) -> ChooseSpec {
    ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface))
}

pub fn this_source_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    leaf::parse_leaf_this_source_reference_words(words)
}

#[cfg(test)]
pub fn tokenize_line(line: &str, line_index: usize) -> Vec<OwnedLexToken> {
    let mut tokens = lex_line(line, line_index).expect("test tokenization helper should lex input");
    for token in &mut tokens {
        token.lowercase_word();
    }
    tokens
}

pub use super::lexer::parser_token_word_refs as words;

pub fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    count_shapes::parse_for_each_count_value_words(words)
}

pub fn is_article(word: &str) -> bool {
    leaf::parse_leaf_article_complete(word).is_ok()
}

pub fn strip_leading_word_refs_any<'slice, 'word>(
    words: &'slice [&'word str],
    leading_words: &[&str],
) -> &'slice [&'word str] {
    let fact = token_facts::strip_leading_selected_word_refs_lexical(words, leading_words);
    &words[fact.consumed_words..]
}

pub fn strip_leading_article_word_refs<'slice, 'word>(
    words: &'slice [&'word str],
) -> &'slice [&'word str] {
    leaf::parse_leaf_leading_articles_words(words).rest
}

pub fn strip_leading_article_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    leaf::parse_leaf_leading_articles_tokens(tokens).rest
}

pub fn strip_leading_token_words_any<'a>(
    tokens: &'a [OwnedLexToken],
    leading_words: &[&str],
) -> &'a [OwnedLexToken] {
    leaf::parse_leaf_leading_selected_tokens(tokens, leading_words).rest
}

pub fn strip_leading_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    strip_leading_article_tokens(tokens).to_vec()
}

pub fn non_article_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    token_facts::non_article_word_refs(words)
}

pub fn word_refs_except<'a>(words: &[&'a str], excluded: &[&str]) -> Vec<&'a str> {
    words
        .iter()
        .copied()
        .filter(|word| !excluded.iter().any(|excluded_word| word == excluded_word))
        .collect()
}

pub fn non_article_word_refs_except<'a>(words: &[&'a str], excluded: &[&str]) -> Vec<&'a str> {
    let words = non_article_word_refs(words);
    word_refs_except(&words, excluded)
}

pub fn non_article_token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    token_facts::non_article_token_word_refs(tokens)
}

pub fn strip_possessive_suffix(word: &str) -> &str {
    leaf::strip_leaf_source_possessive_suffix(word)
}

pub fn possessive_normalized_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
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
pub fn helper_tag_for_tokens(tokens: &[OwnedLexToken], prefix: &str) -> TagKey {
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

pub fn is_sentence_helper_tag(tag: &str, prefix: &str) -> bool {
    let Some(rest) = tag.strip_prefix(SENTENCE_HELPER_TAG_PREFIX) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(prefix) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("_l") else {
        return false;
    };
    let mut parts = rest.split("_s");
    let Some(line) = parts.next() else {
        return false;
    };
    let Some(rest) = parts.next() else {
        return false;
    };
    let mut parts = rest.split("_e");
    let Some(start) = parts.next() else {
        return false;
    };
    let Some(end) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && line.parse::<usize>().is_ok()
        && start.parse::<usize>().is_ok()
        && end.parse::<usize>().is_ok()
}

pub fn classify_instead_followup_tokens(
    tokens: &[OwnedLexToken],
) -> crate::cards::builders::InsteadSemantics {
    super::grammar::effects::classify_instead_followup_semantics_tokens(tokens)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActivationCostObjectReference {
    Tagged(TagKey),
    Source,
}

fn is_activation_cost_object_tag(tag: &TagKey) -> bool {
    [
        SACRIFICE_COST_TAG_PREFIX,
        EXILE_COST_TAG_PREFIX,
        UNATTACH_COST_TAG_PREFIX,
        TAP_COST_TAG_PREFIX,
        RETURN_COST_TAG_PREFIX,
        DISCARD_COST_TAG_PREFIX,
    ]
    .iter()
    .any(|prefix| tag_has_prefix(tag, prefix))
        || tag.as_str() == DISCARDED_COST_TAG
}

fn activation_cost_effect_object_reference(
    effect: &Effect,
) -> Option<ActivationCostObjectReference> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && is_activation_cost_object_tag(&tagged.tag)
    {
        return Some(ActivationCostObjectReference::Tagged(tagged.tag.clone()));
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>()
        && let Some(tag) = discard.tag.as_ref()
    {
        return Some(ActivationCostObjectReference::Tagged(tag.clone()));
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && is_activation_cost_object_tag(&choose.tag)
    {
        return Some(ActivationCostObjectReference::Tagged(choose.tag.clone()));
    }
    if effect
        .downcast_ref::<crate::effects::TapEffect>()
        .is_some_and(|tap| matches!(tap.target.base(), ChooseSpec::Source))
        || effect
            .downcast_ref::<crate::effects::UntapEffect>()
            .is_some_and(|untap| matches!(untap.target.base(), ChooseSpec::Source))
    {
        return Some(ActivationCostObjectReference::Source);
    }
    None
}

fn activation_cost_component_object_reference(
    cost: &crate::costs::Cost,
) -> Option<ActivationCostObjectReference> {
    match cost {
        crate::costs::Cost::Tap
        | crate::costs::Cost::Untap
        | crate::costs::Cost::DiscardSource
        | crate::costs::Cost::SacrificeSelf
        | crate::costs::Cost::ExileSelf
        | crate::costs::Cost::ReturnSelfToHand => Some(ActivationCostObjectReference::Source),
        crate::costs::Cost::Discard { .. } => Some(ActivationCostObjectReference::Tagged(
            TagKey::from(DISCARDED_COST_TAG),
        )),
        crate::costs::Cost::Effect(effect) => activation_cost_effect_object_reference(effect),
        _ => None,
    }
}

fn activation_cost_object_reference(cost: &TotalCost) -> Option<ActivationCostObjectReference> {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => costs
            .iter()
            .filter_map(activation_cost_component_object_reference)
            .next_back(),
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut references = branches.iter().map(activation_cost_object_reference);
            let first = references.next()??;
            references
                .all(|reference| reference.as_ref() == Some(&first))
                .then_some(first)
        }
    }
}

/// Seed resolution references from the last object-bearing activation cost.
///
/// Cost choices run before an activated ability is put on the stack, so an
/// object mentioned by the effect must use the snapshot captured while paying
/// the cost. Source costs use the stack entry's source LKI instead of inventing
/// a global `it` binding.
pub fn activation_cost_reference_imports(cost: &TotalCost) -> ReferenceImports {
    match activation_cost_object_reference(cost) {
        Some(ActivationCostObjectReference::Tagged(tag)) => {
            let mut imports = ReferenceImports::with_last_object_tag(tag.clone());
            if tag.as_str().starts_with("sacrifice_cost_") {
                imports.snapshot_tag_aliases.push((
                    ADDITIONAL_COST_OBJECT_TAG.to_string(),
                    tag.as_str().to_string(),
                ));
            }
            imports
        }
        Some(ActivationCostObjectReference::Source) => ReferenceImports {
            source_object_antecedent: true,
            ..Default::default()
        },
        None => ReferenceImports::default(),
    }
}

fn tag_has_prefix(tag: &TagKey, prefix: &str) -> bool {
    tag.as_str().get(..prefix.len()) == Some(prefix)
}

pub fn value_contains_unbound_x(value: &Value) -> bool {
    match value {
        Value::X | Value::XTimes(_) => true,
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_contains_unbound_x(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_contains_unbound_x(left) || value_contains_unbound_x(right)
        }
        _ => false,
    }
}

pub fn replace_unbound_x_with_value(
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
        Value::SurfaceHinted { value, hints } => {
            let replaced = replace_unbound_x_with_value(*value, replacement, clause)?;
            Ok(replaced.with_surface_hints(hints))
        }
        Value::Scaled(value, multiplier) => Ok(Value::Scaled(
            Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            multiplier,
        )),
        Value::DividedRoundedDown(value, divisor) => Ok(Value::DividedRoundedDown(
            Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            divisor,
        )),
        Value::HalfRoundedDown(value) => Ok(Value::HalfRoundedDown(Box::new(
            replace_unbound_x_with_value(*value, replacement, clause)?,
        ))),
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(replace_unbound_x_with_value(*left, replacement, clause)?),
            Box::new(replace_unbound_x_with_value(*right, replacement, clause)?),
        )),
        Value::Min(left, right) => Ok(Value::Min(
            Box::new(replace_unbound_x_with_value(*left, replacement, clause)?),
            Box::new(replace_unbound_x_with_value(*right, replacement, clause)?),
        )),
        other => Ok(other),
    }
}

pub fn starts_with_activation_cost(tokens: &[OwnedLexToken]) -> bool {
    token_facts::parse_activation_cost_start_tokens(tokens)
        .is_some_and(|fact| fact.token_index == 0)
}

pub fn find_activation_cost_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    token_facts::parse_activation_cost_start_tokens(tokens).map(|fact| fact.token_index)
}

pub fn join_sentences_with_period(sentences: &[Vec<OwnedLexToken>]) -> Vec<OwnedLexToken> {
    let mut joined = Vec::new();
    for (idx, sentence) in sentences.iter().enumerate() {
        if idx > 0 {
            joined.push(OwnedLexToken::period(TextSpan::synthetic()));
        }
        joined.extend(sentence.clone());
    }
    joined
}

pub fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool, PlayerFilter) {
    let Some(facts) = super::grammar::effects::parse_next_end_step_delay_words(tail_words) else {
        return (false, false, PlayerFilter::Any);
    };
    (
        facts.sacrifice_reference,
        facts.exile_reference,
        facts.player,
    )
}

pub fn remove_first_may_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some(fact) = token_facts::parse_first_may_word_token(tokens) else {
        return tokens.to_vec();
    };
    tokens[..fact.token_index]
        .iter()
        .chain(tokens[fact.token_index + 1..].iter())
        .cloned()
        .collect()
}

pub fn remove_through_first_may_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some(fact) = token_facts::parse_first_may_word_token(tokens) else {
        return Vec::new();
    };
    tokens[fact.token_index + 1..].to_vec()
}

pub fn trim_commas(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
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

pub fn trim_edge_punctuation_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

pub fn trim_edge_punctuation(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    trim_edge_punctuation_tokens(tokens).to_vec()
}

pub fn parser_stacktrace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_STACKTRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn parser_trace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn parser_trace(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!(
        "[parser-flow] stage={stage} clause='{}'",
        crate::lexer::token_word_refs(tokens).join(" ")
    );
}

pub fn parser_trace_stack(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_stacktrace_enabled() {
        return;
    }
    eprintln!(
        "[parser-trace] stage={stage} clause='{}'",
        crate::lexer::token_word_refs(tokens).join(" ")
    );
    eprintln!("{}", std::backtrace::Backtrace::force_capture());
}

pub fn map_span_to_original(
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
    let end_orig = if end_char == 0 || end_char > char_map.len() {
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

pub fn parse_card_type(word: &str) -> Option<CardType> {
    leaf::parse_leaf_card_type_complete(word).ok()
}

pub fn parse_supertype_word(word: &str) -> Option<Supertype> {
    leaf::parse_leaf_supertype_complete(word).ok()
}

pub fn parse_subtype_word(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_subtype_complete(word).ok()
}

pub fn parse_mana_symbol_word_flexible(word: &str) -> Option<ManaSymbol> {
    leaf::parse_leaf_spelled_mana_word_complete(word).ok()
}

pub fn parse_color(word: &str) -> Option<crate::color::ColorSet> {
    leaf::parse_leaf_color_complete(word).ok()
}

pub fn parse_non_type(word: &str) -> Option<CardType> {
    leaf::parse_leaf_non_card_type_complete(word).ok()
}

pub fn parse_non_supertype(word: &str) -> Option<Supertype> {
    leaf::parse_leaf_non_supertype_complete(word).ok()
}

pub fn parse_non_color(word: &str) -> Option<crate::color::ColorSet> {
    leaf::parse_leaf_non_color_complete(word).ok()
}

pub fn parse_non_subtype(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_non_subtype_complete(word).ok()
}

pub fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
    leaf::parse_leaf_subtype_flexible_complete(word).ok()
}

pub fn is_source_reference_words(words: &[&str]) -> bool {
    is_this_source_reference_words(words) || source_reference_surface_for_words(words).is_some()
}

fn is_this_source_reference_words(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
}

pub fn is_demonstrative_object_head(word: &str) -> bool {
    leaf::parse_leaf_demonstrative_object_head_complete(word).is_ok()
}

pub fn is_outlaw_word(word: &str) -> bool {
    token_facts::parse_outlaw_word(word) == Some(token_facts::OutlawWord::Outlaw)
}

pub fn is_non_outlaw_word(word: &str) -> bool {
    token_facts::parse_outlaw_word(word) == Some(token_facts::OutlawWord::NonOutlaw)
}

pub fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
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

pub fn is_permanent_type(card_type: CardType) -> bool {
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

pub fn parse_zone_word(word: &str) -> Option<Zone> {
    leaf::parse_leaf_zone_complete(word).ok()
}

pub fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    let parsed = leaf::parse_leaf_alternative_cast_prefix_words(words)?;
    Some((parsed.kind, parsed.consumed))
}

pub fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    leaf::parse_leaf_unsigned_pt_complete(word).ok()
}

pub fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    reference_shapes::parse_filter_keyword_constraint_words(words)
}

pub use reference_shapes::FilterKeywordListConnective;

pub fn parse_filter_keyword_constraint_list_words(
    words: &[&str],
) -> Option<(
    Vec<FilterKeywordConstraint>,
    FilterKeywordListConnective,
    usize,
)> {
    reference_shapes::parse_filter_keyword_constraint_list_words(words)
}

/// Return whether the words after a comma continue a keyword predicate list.
///
/// This is deliberately a boundary fact rather than a filter parser: callers
/// use it before splitting trigger and effect clauses, when a tail such as
/// `double strike, vigilance, or haste` must remain attached to the filter on
/// the left.  A final list arm begins with its connective, so one parsed
/// keyword is sufficient in that case; otherwise require at least two parsed
/// keywords to distinguish a serial list from an independent keyword action.
pub fn starts_filter_keyword_list_continuation_words(words: &[&str]) -> bool {
    let (has_leading_connective, keyword_words) = match words {
        ["and", "or", rest @ ..] => (true, rest),
        ["and" | "or" | "and/or", rest @ ..] => (true, rest),
        _ => (false, words),
    };
    let Some((constraints, _, _consumed)) =
        parse_filter_keyword_constraint_list_words(keyword_words)
    else {
        return false;
    };
    has_leading_connective || constraints.len() > 1
}

pub fn apply_filter_keyword_constraint(
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

pub fn parse_flashback_keyword_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
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

pub fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    shared_tokens::parse_mana_symbol(part)
}

pub fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_tokens::parse_scryfall_mana_cost(raw)
}

pub fn parse_number_word_i32(word: &str) -> Option<i32> {
    leaf::parse_number_i32_complete(word).ok()
}

pub fn parse_number_word_u32(word: &str) -> Option<u32> {
    parse_number_word_i32(word).and_then(|value| value.try_into().ok())
}

pub fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    value_expr::parse_value_expr_words(words)
}

pub fn parse_value_expr(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    value_expr::parse_value_expr_tokens(tokens)
}

pub fn parse_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_value_expr(tokens)
}

pub fn parse_subject(tokens: &[OwnedLexToken]) -> SubjectAst {
    reference_shapes::parse_subject_tokens(tokens)
}

pub fn span_from_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    token_slice_span(tokens)
}

pub fn parse_number(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    leaf::parse_leaf_number_prefix_tokens(tokens)?.into_fixed()
}

pub fn parse_quantity_comparison_prefix(
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

pub fn parse_quantity_comparison_prefix_words(
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

pub fn comparison_to_at_least_threshold(comparison: &crate::effect::Comparison) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        crate::effect::Comparison::Equal(value) if *value >= 0 => Some(*value as u32),
        _ => None,
    }
}

pub fn comparison_to_strict_at_least_threshold(
    comparison: &crate::effect::Comparison,
) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        _ => None,
    }
}

pub fn comparison_to_strict_at_most_threshold(
    comparison: &crate::effect::Comparison,
) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::LessThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::LessThan(value) if *value > 0 => Some((*value - 1) as u32),
        _ => None,
    }
}

pub fn comparison_to_value_comparison_operator(
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

pub fn parse_greater_than_or_equal_quantity_prefix(
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

pub fn parse_less_than_or_equal_quantity_prefix(
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

pub fn parse_choice_count_token_prefix_consumed(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    Some((parsed.count, parsed.consumed))
}

pub fn parse_choice_count_before_target_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let fact = token_facts::parse_choice_count_before_target_tokens(tokens)?;
    Some((fact.count, fact.consumed_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn source_surface_recording_restores_canonical_alias_casing_only_for_exact_aliases() {
        let aliases = source_reference_aliases_for_name("Ghyrson Starn, Kelermorph");
        assert_eq!(
            canonical_source_reference_surface(
                &aliases,
                SourceReferenceSurface::ShortName("ghyrson Starn".to_string()),
            ),
            SourceReferenceSurface::ShortName("Ghyrson Starn".to_string())
        );
        assert_eq!(
            canonical_source_reference_surface(
                &aliases,
                SourceReferenceSurface::FullName("ghyrson Starn".to_string()),
            ),
            SourceReferenceSurface::ShortName("Ghyrson Starn".to_string())
        );
        assert_eq!(
            canonical_source_reference_surface(
                &aliases,
                SourceReferenceSurface::ShortName("another source".to_string()),
            ),
            SourceReferenceSurface::ShortName("another source".to_string())
        );
    }

    #[test]
    fn replacing_surface_hinted_x_flattens_replacement_hints() {
        let value = Value::X
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence);
        let replacement =
            Value::Fixed(3).with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);

        let replaced =
            replace_unbound_x_with_value(value, &replacement, "focused hint merge").unwrap();

        assert!(matches!(replaced.unhinted(), Value::Fixed(3)));
        assert!(replaced.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
        assert!(
            replaced.has_surface_hint(
                ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence
            )
        );
        assert!(
            matches!(
                replaced,
                Value::SurfaceHinted { ref value, .. }
                    if !matches!(value.as_ref(), Value::SurfaceHinted { .. })
            ),
            "merged hints must use a single surface wrapper: {replaced:#?}"
        );
    }

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
    fn parse_subject_recognizes_they_after_leading_instead() {
        let tokens = lex_line("instead they exile the top card of their library", 0).unwrap();
        assert_eq!(parse_subject(&tokens), SubjectAst::Player(PlayerAst::That));
    }

    #[test]
    fn parse_subject_preserves_seat_relative_players() {
        let right = lex_line("the player to your right gains control of this artifact", 0).unwrap();
        assert_eq!(
            parse_subject(&right),
            SubjectAst::Player(PlayerAst::PlayerToYourRight)
        );

        let left = lex_line("the player to your left chooses a color", 0).unwrap();
        assert_eq!(
            parse_subject(&left),
            SubjectAst::Player(PlayerAst::PlayerToYourLeft)
        );
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
    fn parse_target_phrase_preserves_graveyard_entry_history() {
        for (text, from_battlefield) in [
            (
                "target nonland card in a graveyard that was put there from anywhere this turn",
                false,
            ),
            (
                "target creature card in your graveyard that was put there from the battlefield this turn",
                true,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let target =
                parse_target_phrase(&tokens).expect("temporal graveyard target should parse");
            let TargetAst::Object(filter, target_span, _) = target else {
                panic!("expected target object for {text}, got {target:?}");
            };
            assert!(target_span.is_some(), "expected explicit target span");
            assert!(
                filter.entered_graveyard_this_turn,
                "graveyard-entry history was dropped for {text}: {filter:#?}"
            );
            assert_eq!(
                filter.entered_graveyard_from_battlefield_this_turn, from_battlefield,
                "{text}: {filter:#?}"
            );
        }
    }

    #[test]
    fn parse_target_phrase_preserves_opponent_library_to_graveyard_history() {
        let text = "target artifact or creature card in an opponent's graveyard that was put there from their library this turn";
        let tokens = lex_line(text, 0).unwrap();
        let target = parse_target_phrase(&tokens)
            .expect("opponent library-to-graveyard target should parse");
        let TargetAst::Object(filter, target_span, _) = target else {
            panic!("expected target object, got {target:?}");
        };
        assert!(target_span.is_some());
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::Opponent));
        assert_eq!(filter.card_types, [CardType::Artifact, CardType::Creature]);
        assert!(filter.entered_graveyard_this_turn, "{filter:#?}");
        assert!(
            filter.entered_graveyard_from_library_this_turn,
            "{filter:#?}"
        );
        assert!(filter.any_of.is_empty(), "{filter:#?}");
    }

    #[test]
    fn parse_target_phrase_preserves_distinct_combat_damage_controller_history() {
        let tokens = lex_line(
            "target nonland permanent controlled by a player who was dealt combat damage by three or more Pirates this turn",
            0,
        )
        .expect("historical controller target should lex");
        let TargetAst::Object(filter, explicit_target, _) =
            parse_target_phrase(&tokens).expect("historical controller target should parse")
        else {
            panic!("expected an object target");
        };
        assert!(explicit_target.is_some());
        assert_eq!(filter.excluded_card_types, [CardType::Land]);
        let Some(PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        }) = filter.controller.as_ref()
        else {
            panic!("historical controller relation was lost: {filter:#?}");
        };
        assert_eq!(base.as_ref(), &PlayerFilter::Any);
        assert_eq!(*minimum, 3);
        assert_eq!(sources.subtypes, [Subtype::Pirate]);
    }

    #[test]
    fn parse_target_phrase_preserves_drafted_color_qualifier_instead_of_card_name() {
        let tokens = lex_line(
            "target creature that's one or more of the colors chosen as you drafted cards named Regicide",
            0,
        )
        .expect("drafted-color target should lex");
        let TargetAst::Object(filter, explicit_target, _) =
            parse_target_phrase(&tokens).expect("drafted-color target should parse")
        else {
            panic!("expected an object target");
        };
        assert!(explicit_target.is_some());
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.name, None);
        assert_eq!(
            filter.colors_chosen_while_drafting_named.as_deref(),
            Some("Regicide")
        );

        let named_tokens = lex_line("target creature card named Regicide", 0)
            .expect("ordinary named-card target should lex");
        let TargetAst::Object(named_filter, _, _) =
            parse_target_phrase(&named_tokens).expect("ordinary named target should parse")
        else {
            panic!("expected an ordinary named object target");
        };
        assert_eq!(named_filter.name.as_deref(), Some("Regicide"));
        assert!(named_filter.colors_chosen_while_drafting_named.is_none());
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
        assert!(
            matches!(
                value.unhinted(),
                Value::PendingPriorEffectMetric(query)
                    if query.action == Some(ironsmith_core::PriorEffectAction::Revealed)
            ),
            "expected a typed revealed-this-way metric, got {value:?}"
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
        assert!(
            filter.controller.is_some(),
            "expected controller filter, got {filter:?}"
        );
        assert_eq!(filter.any_of.len(), 3, "{filter:?}");
        assert!(
            filter.any_of.iter().all(|branch| branch.untapped),
            "the shared untapped qualifier belongs on every union branch: {filter:?}"
        );
        assert!(
            [CardType::Artifact, CardType::Creature, CardType::Land]
                .into_iter()
                .all(|card_type| filter
                    .any_of
                    .iter()
                    .any(|branch| branch.card_types == [card_type])),
            "the three authored type branches should remain distinct: {filter:?}"
        );
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
        let eternalize_with_additional_cost =
            lex_line("Eternalize—{2}{W}{W}, Discard a card.", 0).unwrap();
        let eternalize_with_additional_cost =
            parse_eternalize_line_lexed(&eternalize_with_additional_cost)
                .unwrap()
                .expect("eternalize with a compound activation cost should parse");
        assert_eq!(eternalize_with_additional_cost.costs().len(), 2);
        assert!(eternalize_with_additional_cost.costs().iter().any(|cost| {
            matches!(
                cost,
                crate::costs::Cost::Discard {
                    count: 1,
                    card_types
                } if card_types.is_empty()
            )
        }));

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
                    ChooseSpecSurfaceHint::SacrificedObject(_) => None,
                })
                .unwrap_or_else(|| format!("{spec:?}")),
            _ => format!("{spec:?}"),
        }
    }
}

pub fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

fn with_leading_object_set_quantifier(
    mut target: TargetAst,
    surface: Option<ironsmith_core::SetQuantifierSurface>,
) -> TargetAst {
    let Some(surface) = surface else {
        return target;
    };

    fn apply(target: &mut TargetAst, surface: ironsmith_core::SetQuantifierSurface) {
        match target {
            TargetAst::Object(filter, _, _) => filter.set_set_quantifier_surface(Some(surface)),
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                apply(inner, surface)
            }
            _ => {}
        }
    }

    apply(&mut target, surface);
    target
}

/// Preserve a controller-history qualifier that belongs to the target's
/// object filter. The generic target grammar also recognizes trailing
/// controller-set constraints ("controlled by different players"); recover
/// this semantically different singular relation from the complete raw target
/// tail before those set-level suffixes can narrow the filter surface.
fn restore_distinct_combat_damage_controller_target(
    target: &mut TargetAst,
    tokens: &[OwnedLexToken],
) {
    let Ok(head) = leaf::parse_leaf_target_head_tokens(tokens) else {
        return;
    };
    let Some(filter_tokens) = head.tokens().get(head.prefix.consumed..) else {
        return;
    };
    let Ok(authored_filter) = crate::object_filters::parse_object_filter(filter_tokens, false)
    else {
        return;
    };
    if !matches!(
        authored_filter.controller,
        Some(PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. })
    ) {
        return;
    }

    fn apply(target: &mut TargetAst, authored_filter: &ObjectFilter) {
        match target {
            TargetAst::Object(filter, _, _) => {
                // The complete raw filter is the authoritative typed parse.
                // The ordinary target envelope can represent `permanent` as
                // an expanded card-type set while the family parser keeps it
                // implicit, so an equality check between those equivalent
                // internal encodings incorrectly rejected this recovery. It
                // could also consume the terminal source kind ("Pirates") as
                // a target subtype. Replacing only after the full raw filter
                // has proven the exact historical-controller variant keeps
                // both the target domain and the source predicate together.
                *filter = authored_filter.clone();
            }
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                apply(inner, authored_filter)
            }
            _ => {}
        }
    }

    apply(target, &authored_filter);
}

pub fn parse_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    // A plural historical graveyard target has an embedded `put` verb that
    // belongs to the object filter, not to the surrounding action chain.
    // Preserve this exact target envelope before the generic target-head
    // parser can expose that verb as a second effect clause.
    let historical_words = crate::lexer::token_word_refs(tokens);
    if matches!(
        historical_words.as_slice(),
        [
            "up",
            "to",
            "three" | "3",
            "target",
            "permanent",
            "cards",
            "in",
            "graveyards",
            "that",
            "were",
            "put",
            "there",
            "from",
            "the",
            "battlefield",
            "this",
            "turn"
        ] | [
            "up",
            "to",
            "three" | "3",
            "target",
            "permanent",
            "cards",
            "in",
            "graveyards",
            "that",
            "were",
            "put",
            "there",
            "from",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        let mut filter = ObjectFilter::permanent_card().in_zone(Zone::Graveyard);
        filter.entered_graveyard_this_turn = true;
        filter.entered_graveyard_from_battlefield_this_turn = true;
        filter.set_graveyard_entry_history_surface(Some(
            ironsmith_core::GraveyardEntryHistorySurface::PutThereFromBattlefieldThisTurn,
        ));
        return Ok(TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, span_from_tokens(tokens), None)),
            ChoiceCount::up_to(3),
        ));
    }

    // A bare object AST does not itself distinguish the chosen singleton in
    // "a creature" from the complete set in "each/all creature". Preserve
    // that authored quantifier on the filter so action lowering can retain
    // complete-set semantics even when the phrase also references a chosen
    // player (for example, "each creature target player controls").
    let leading_set_quantifier = match crate::lexer::token_word_refs(tokens).first() {
        Some(&"each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        Some(&"all") => Some(ironsmith_core::SetQuantifierSurface::All),
        _ => None,
    };
    let envelope = parse_target_envelope(tokens);
    if let Some(count) = envelope.counted_any_target {
        return Ok(with_leading_object_set_quantifier(
            TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(tokens))),
                count,
            ),
            leading_set_quantifier,
        ));
    }

    let mut target = match parse_target_phrase_inner(tokens) {
        Ok(target) => target,
        Err(err) => {
            for candidate in envelope.recovery_candidates {
                if let Ok(mut target) = parse_target_phrase_inner(candidate.tokens) {
                    restore_distinct_combat_damage_controller_target(&mut target, tokens);
                    return Ok(with_leading_object_set_quantifier(
                        target,
                        leading_set_quantifier,
                    ));
                }
            }
            return Err(err);
        }
    };
    restore_distinct_combat_damage_controller_target(&mut target, tokens);
    Ok(with_leading_object_set_quantifier(
        target,
        leading_set_quantifier,
    ))
}

fn parse_target_phrase_inner(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    target_semantics::parse_target_phrase_inner(tokens)
}

pub fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, Option<String>, String)> {
    let parsed = header_shapes::parse_saga_chapter_header(line)?;
    Some((parsed.chapters, parsed.presentation_label, parsed.body))
}

pub fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let parsed = header_shapes::parse_level_header(line)?;
    Some((parsed.minimum, parsed.maximum))
}

pub fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    leaf::parse_leaf_power_toughness_complete(raw).ok()
}

pub fn parse_level_up_line(
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

pub fn parse_level_up_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_level_up_line(tokens)
}

pub fn parse_self_free_cast_alternative_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    alternative_cost_lines::parse_self_free_cast(tokens)
}

pub fn parse_self_free_cast_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_self_free_cast_alternative_cost_line(tokens)
}

pub fn parse_flash_with_additional_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    alternative_cost_lines::parse_flash_with_additional_cost(tokens)
}

pub fn parse_flash_with_additional_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_flash_with_additional_cost_line(tokens)
}

pub fn mana_pips_from_token(token: &OwnedLexToken) -> Option<Vec<ManaSymbol>> {
    leaf::parse_leaf_surface_mana_pip_token(token).map(leaf::LeafManaPipToken::into_pip)
}

pub fn leading_mana_cost_from_tokens(tokens: &[OwnedLexToken]) -> Option<(ManaCost, usize)> {
    let prefix = leaf::parse_leaf_mana_cost_prefix_tokens(tokens)?;
    Some((prefix.cost, prefix.consumed))
}

pub fn parse_madness_line(
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

pub fn parse_madness_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_madness_line(tokens)
}

pub fn parse_buyback_line(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
    keyword_cost_lines::parse_buyback(tokens)
}

pub fn parse_buyback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_buyback_line(tokens)
}

pub fn parse_bargain_line(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
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

pub fn parse_bargain_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_bargain_line(tokens)
}

pub fn parse_optional_cost_keyword_line(
    tokens: &[OwnedLexToken],
    keyword: &str,
    constructor: fn(TotalCost) -> OptionalCost,
) -> Result<Option<OptionalCost>, CardTextError> {
    keyword_cost_lines::parse_optional_cost(tokens, keyword, constructor)
}

pub fn parse_kicker_line(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "kicker", OptionalCost::kicker)
}

pub fn parse_kicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_kicker_line(tokens)
}

pub fn parse_multikicker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "multikicker", OptionalCost::multikicker)
}

pub fn parse_multikicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_multikicker_line(tokens)
}

pub fn parse_replicate_line(
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

pub fn parse_replicate_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_replicate_line(tokens)
}

pub fn parse_squad_line(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "squad", OptionalCost::squad)
}

pub fn parse_squad_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_squad_line(tokens)
}

pub fn parse_offspring_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "offspring", OptionalCost::offspring)
}

pub fn parse_offspring_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_offspring_line(tokens)
}

pub fn parse_entwine_line(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "entwine", OptionalCost::entwine)
}

pub fn parse_entwine_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_entwine_line(tokens)
}

pub fn parse_escalate_line_lexed(
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

pub fn parse_evoke_line_lexed(
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
        name: "Evoke".into(),
        total_cost,
        condition: None,
        prototype_power_toughness: None,
    }))
}

pub fn parse_prowl_line_lexed(
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
        name: "Prowl".into(),
        total_cost,
        condition: Some(
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue,
            ),
        ),
        prototype_power_toughness: None,
    }))
}

pub fn parse_eternalize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<TotalCost>, CardTextError> {
    let Some(fact) =
        keyword_line_facts::parse_named_cost_line_tokens(tokens, NamedCostKeyword::Eternalize)
    else {
        return Ok(None);
    };
    if fact.cost_tokens.is_empty() {
        return Ok(None);
    }
    let total_cost = parse_activation_cost(fact.cost_tokens)?;
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "eternalize keyword missing mana cost".to_string(),
        ));
    }
    Ok(Some(total_cost))
}

pub fn parse_epic_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    keyword_line_facts::parse_epic_line_tokens(tokens).is_some()
}

pub fn parse_morph_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    keyword_cost_lines::parse_morph(tokens)
}

pub fn parse_morph_keyword_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_morph_keyword_line(tokens)
}

pub fn parse_escape_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    keyword_cost_lines::parse_escape(tokens)
}

pub fn parse_escape_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_escape_line(tokens)
}

pub fn parse_flashback_line(
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

    let total_cost = match parse_leading_mana_and_payment_total_cost(cost_tokens)? {
        Some(total_cost) => total_cost,
        None => match parse_activation_cost(cost_tokens) {
            Ok(total_cost) => total_cost,
            Err(_) => {
                crate::activation_and_restrictions::parse_payment_clause_as_total_cost(cost_tokens)?
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported flashback cost (clause: '{}')",
                            words(cost_tokens).join(" ")
                        ))
                    })?
            }
        },
    };

    Ok(Some(AlternativeCastingMethod::Flashback { total_cost }))
}

/// Parse an alternative cost whose leading mana symbols are followed by a
/// typed nonmana payment, such as `{R}{R}, discard X cards`.  The ordinary
/// payment-clause fallback intentionally ignores non-payment prose, but that
/// also means it can return only the trailing effect cost.  Prove and retain
/// both halves here before that fallback runs.
fn parse_leading_mana_and_payment_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<TotalCost>, CardTextError> {
    let Some((mana, consumed)) = leading_mana_cost_from_tokens(tokens) else {
        return Ok(None);
    };
    let tail = trim_commas(tokens.get(consumed..).unwrap_or_default());
    if tail.is_empty() {
        return Ok(None);
    }
    let Some(nonmana) =
        crate::activation_and_restrictions::parse_payment_clause_as_total_cost(&tail)?
    else {
        return Ok(None);
    };
    let Some(nonmana_costs) = nonmana.as_all() else {
        return Ok(None);
    };
    if nonmana_costs.is_empty() || nonmana_costs.iter().any(|cost| cost.is_mana_cost()) {
        return Ok(None);
    }
    let mut costs = Vec::with_capacity(nonmana_costs.len() + 1);
    costs.push(Cost::mana(mana));
    costs.extend(nonmana_costs.iter().cloned());
    Ok(Some(TotalCost::from_costs(costs)))
}

#[cfg(test)]
mod mixed_flashback_cost_tests {
    use super::*;

    #[test]
    fn flashback_keeps_leading_mana_and_dynamic_discard_payment() {
        let tokens = lex_line("Flashback—{R}{R}, Discard X cards.", 0)
            .expect("mixed flashback cost should lex");
        let method = parse_flashback_line(&tokens)
            .expect("mixed flashback cost should parse")
            .expect("flashback should be recognized");
        let AlternativeCastingMethod::Flashback { total_cost } = method else {
            panic!("expected flashback alternative cost: {method:#?}");
        };
        assert_eq!(
            total_cost
                .mana_cost()
                .expect("leading mana component must survive")
                .to_oracle(),
            "{R}{R}"
        );
        let debug = format!("{:#?}", total_cost.non_mana_costs().collect::<Vec<_>>());
        assert!(debug.contains("DiscardEffect"), "{debug}");
        assert!(debug.contains("count: X"), "{debug}");
    }
}

pub fn parse_flashback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_flashback_line(tokens)
}

pub fn parse_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if keyword_line_facts::parse_retrace_line_tokens(tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(AlternativeCastingMethod::Retrace {
        total_cost: TotalCost::from_cost(Cost::discard(1, Some(CardType::Land))),
    }))
}

pub fn parse_retrace_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_retrace_line(tokens)
}

pub fn parse_jump_start_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    Ok(keyword_cost_lines::parse_jump_start(tokens))
}

pub fn parse_jump_start_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_jump_start_line(tokens)
}

pub fn parse_harmonize_line(
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

pub fn parse_harmonize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_harmonize_line(tokens)
}

pub fn parse_warp_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let Some(fact) = keyword_line_facts::parse_warp_line_tokens(tokens) else {
        return Ok(None);
    };
    let (cost, _) = leading_mana_cost_from_tokens(fact.cost_tokens)
        .ok_or_else(|| CardTextError::ParseError("warp keyword missing mana cost".to_string()))?;
    Ok(Some(AlternativeCastingMethod::Warp { cost }))
}

pub fn parse_warp_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_warp_line(tokens)
}

pub fn parse_bestow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    keyword_cost_lines::parse_bestow(tokens)
}

pub fn parse_bestow_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_bestow_line(tokens)
}

pub fn parse_blitz_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    keyword_cost_lines::parse_blitz(tokens)
}

pub fn parse_blitz_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_blitz_line(tokens)
}

pub fn parse_transmute_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    keyword_cost_lines::parse_transmute(tokens)
}

pub fn parse_transmute_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_transmute_line(tokens)
}

pub fn parse_transfigure_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    keyword_cost_lines::parse_transfigure(tokens)
}

pub fn parse_transfigure_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_transfigure_line(tokens)
}

pub fn parse_reinforce_line(
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

pub fn parse_reinforce_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_reinforce_line(tokens)
}

pub fn parse_cast_this_spell_only_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    cast_restriction_lines::parse_cast_this_spell_only(tokens)
}

pub fn parse_cast_this_spell_only_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    parse_cast_this_spell_only_line(tokens)
}

pub fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    alternative_cost_lines::parse_you_may_rather_than_spell_cost(tokens, line)
}

pub fn parse_you_may_rather_than_spell_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_you_may_rather_than_spell_cost_line(tokens, line)
}

pub fn parse_additional_cost_choice_options(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst<crate::model::ast::EffectAst>>>, CardTextError>
{
    additional_cost_choices::parse_additional_cost_choices(tokens)
}

pub fn parse_additional_cost_choice_options_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst<crate::model::ast::EffectAst>>>, CardTextError>
{
    parse_additional_cost_choice_options(tokens)
}

pub fn parse_if_conditional_alternative_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    alternative_cost_lines::parse_if_conditional_alternative_cost(tokens, line)
}

pub fn parse_if_conditional_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_if_conditional_alternative_cost_line(tokens, line)
}
