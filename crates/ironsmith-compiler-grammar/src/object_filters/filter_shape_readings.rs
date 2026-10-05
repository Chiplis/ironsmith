//! The readings of one object filter phrase before the characteristic
//! grammar: a distinct-combat-damage controller, a trailing "where X" clause,
//! explicit card disjunctions, subtype-or-color disjunctions, selector unions,
//! branch-scoped unions, a generic card tail. Formerly a first-match ladder in
//! `object_filters`; every reading runs, resolved by rank while the overlaps
//! are measured. The characteristic grammar is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct FilterPhrase<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) other: bool,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl FilterPhrase<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<ObjectFilter>, CardTextError>,
    ) -> ParseOutcome<ObjectFilter> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("object-filter-inner-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&FilterPhrase<'_>) -> bool,
    read: fn(&FilterPhrase<'_>) -> ParseOutcome<ObjectFilter>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("object-filter-inner-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("quantified-spell-cost-or-target-suffix"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quantified_spell_suffix(input.tokens, input.other)),
    },
    Reading {
        id: RuleId::new("distinct-combat-damage-controller"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_distinct_combat_damage_controller(input)),
    },
    Reading {
        id: RuleId::new("trailing-where-x-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_trailing_where_x_clause(input)),
    },
    Reading {
        id: RuleId::new("explicit-card-filter-disjunction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("trailing-where-x-clause")
        },
        read: |input| input.outcome(read_explicit_card_filter_disjunction(input)),
    },
    Reading {
        id: RuleId::new("subtype-or-colored-permanent-disjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_subtype_or_colored_permanent_disjunction(input)),
    },
    Reading {
        id: RuleId::new("repeated-selector-domain-union"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_selector_domain_union(input)),
    },
    Reading {
        id: RuleId::new("branch-scoped-union"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("explicit-card-filter-disjunction")
        },
        read: |input| input.outcome(read_branch_scoped_union(input)),
    },
    Reading {
        id: RuleId::new("generic-card-tail-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_card_tail_filter(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &FilterPhrase<'_>) -> ParseOutcome<RuleMatch<ObjectFilter>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) if reading.id.as_str() == "quantified-spell-cost-or-target-suffix" => {
                return ParseOutcome::Error(diagnostic);
            }
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<ObjectFilter>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_distinct_combat_damage_controller(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    if let Some((base_tokens, source_tokens, minimum)) =
        split_distinct_combat_damage_controller_tokens(tokens)
    {
        let mut filter = parse_object_filter(&base_tokens, other)?;
        let sources = parse_object_filter(&source_tokens, false)?;
        filter.controller = Some(
            PlayerFilter::was_dealt_combat_damage_by_distinct_sources_this_turn(
                PlayerFilter::Any,
                sources,
                minimum,
            ),
        );
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_trailing_where_x_clause(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    // The surrounding sentence owns an authored `where X is ...` binding.
    // Keep that definition out of the object-domain grammar: characteristic
    // words in the value expression (for example, `Shrines you control`) are
    // not additional characteristics of the targeted object. The sentence
    // binder subsequently replaces the comparison's typed `Value::X`.
    if let Some(base_tokens) = split_trailing_where_x_filter_clause(tokens) {
        return parse_object_filter(base_tokens, other).map(Some);
    }
    Ok(None)
}
fn read_explicit_card_filter_disjunction(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    if let Some(filter) = parse_explicit_card_filter_disjunction(tokens, other)? {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_subtype_or_colored_permanent_disjunction(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    if let Some(filter) = parse_subtype_or_colored_permanent_disjunction(tokens, other) {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_repeated_selector_domain_union(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    let has_shared_terminal_noun = has_shared_terminal_object_noun(tokens);
    if has_shared_terminal_noun
        && let Some(filter) = parse_repeated_selector_domain_union_lexed(tokens, other)
    {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_branch_scoped_union(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    let other = input.other;
    // "spell that targets an artifact or creature you control" (Fugitive
    // Droid): the disjunction belongs to the targeting clause, not to the
    // spell selector.
    if disjunction_is_inside_targets_clause(tokens) {
        return Ok(None);
    }
    let has_shared_terminal_noun = has_shared_terminal_object_noun(tokens);
    let repeats_card_noun = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .filter(|word| matches!(*word, "card" | "cards"))
        .count()
        >= 2;
    if (!has_shared_terminal_noun || has_requantified_comma_collection(tokens) || repeats_card_noun)
        && let Some(filter) = parse_branch_scoped_object_filter_union_lexed(tokens, other)
    {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_generic_card_tail_filter(
    input: &FilterPhrase<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    if let Some(filter) = parse_generic_card_tail_filter(tokens) {
        return Ok(Some(filter));
    }
    Ok(None)
}

/// Whether every "or" of the phrase follows a "that targets" relative clause,
/// so the disjunction describes the targeted objects.
pub(super) fn disjunction_is_inside_targets_clause(tokens: &[OwnedLexToken]) -> bool {
    // "Aura attached to a creature or land" (Enchantment Alteration): the
    // disjunction names what the Aura is attached to, so it belongs to the
    // attachment clause rather than splitting the Aura selector.
    let Some(targets) = tokens
        .windows(2)
        .position(|window| window[0].is_word("that") && window[1].is_any_word(&["targets", "target"]))
        .or_else(|| {
            tokens
                .windows(2)
                .position(|window| window[0].is_word("attached") && window[1].is_word("to"))
        })
    else {
        return false;
    };
    let mut ors = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_word("or"))
        .map(|(index, _)| index)
        .peekable();
    ors.peek().is_some() && ors.all(|index| index > targets)
}

/// The suffix owns its count and color before the noun reader can mistake
/// `blue mana symbols` for a blue spell or discard a bare target arity.
pub(super) fn read_quantified_spell_suffix(tokens: &[OwnedLexToken], other: bool) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((with, count, tail)) = tokens.iter().enumerate().find_map(|(with, token)| {
        if !token.is_word("with") { return None; }
        let (count, tail) = crate::grammar::primitives::parse_prefix(
            &tokens[with + 1..], crate::grammar::leaf::parse_leaf_choice_count_prefix_lexed,
        )?;
        Some((with, count, tail))
    }) else { return Ok(None); };
    if count.dynamic_x || count.random { return Ok(None); }
    // Every tail token belongs to this grammar, including punctuation and
    // braced mana symbols. The diagnostic view may be lossy; admission may not.
    let complete_words = tail.iter().map(OwnedLexToken::as_word).collect::<Option<Vec<_>>>();
    let words = complete_words.as_deref().unwrap_or(&[]);
    let diagnostic_words = crate::lexer::parser_token_word_refs(tail);
    let targets = matches!(words, ["target"] | ["targets"]);
    let color = if let [color, "mana", "symbols", "in", "its", "mana", "cost"] = words {
        crate::color::Color::from_name(color)
    } else { None };
    if !targets && color.is_none() {
        if diagnostic_words.first().is_some_and(|word| matches!(*word, "target" | "targets"))
            || (diagnostic_words.get(1) == Some(&"mana") && diagnostic_words.get(2) == Some(&"symbols")) {
            return Err(CardTextError::ParseError("unsupported complete quantified spell suffix".into()));
        }
        return Ok(None);
    }
    let mut filter = parse_object_filter(&tokens[..with], other)?;
    if targets { filter.target_count = Some(count); }
    if let Some(color) = color { filter.mana_symbol_count = Some((color, count)); }
    Ok(Some(filter))
}

#[cfg(test)]
mod cast_quantity_filter_tests {
    use super::*;

    #[test]
    fn complete_cost_symbol_suffix_owns_its_color_and_count() {
        for color in crate::color::Color::ALL {
            let text = format!("a noncreature spell with one or more {} mana symbols in its mana cost", color.name());
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let filter = parse_object_filter(&tokens, false).unwrap();
            assert_eq!(filter.mana_symbol_count, Some((color, crate::effect::ChoiceCount::at_least(1))));
            assert!(filter.colors.is_none(), "the symbol color is not a spell color");
            assert!(filter.excluded_card_types.contains(&crate::types::CardType::Creature));
            assert!(filter.description().contains(&format!("{} mana symbols", color.name())));
        }
    }

    #[test]
    fn complete_bare_target_arity_survives_the_noun_reader() {
        let tokens = crate::lexer::lex_line("a spell with one or more targets", 0).unwrap();
        let filter = parse_object_filter(&tokens, false).unwrap();
        assert_eq!(filter.target_count, Some(crate::effect::ChoiceCount::at_least(1)));
        assert!(filter.targets_object.is_none() && filter.targets_player.is_none());
    }

    #[test]
    fn quantitative_reader_requires_the_entire_suffix() {
        for text in [
            "a spell with one or more blue mana symbols",
            "a spell with one or more blue mana symbols in its mana cost and an unknown restriction",
            "a spell with one or more targets and an unknown restriction",
            "a spell with one or more blue mana symbols in its mana cost {R}",
            "a spell with one or more targets:",
            "a spell with one or more blue mana symbols in its mana cost with an unknown restriction",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(read_quantified_spell_suffix(&tokens, false).is_err());
            assert!(parse_object_filter(&tokens, false).is_err(), "{text}");
            assert!(crate::object_filters::parse_object_filter_lexed(&tokens, false).is_err(), "{text}");
        }
    }
}
