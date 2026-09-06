//! The readings of one "pay ..." clause: any amount of energy or life, one or
//! more, the compound payments, the repeated tagged mana payment, "for each"
//! mana, half life, a life or energy amount, the energy-count clauses. Formerly
//! a first-match ladder in `resource`; every reading runs, resolved by rank
//! while the overlaps are measured. The mana-pip payment is the fallback.

use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ManaActionAst;

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct PayClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) player: PlayerAst,
    pub(super) energy_symbol_count: usize,
    pub(super) clause_words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl PayClause<'_> {
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
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("pay-clause-registry-reading"),
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
    admits: fn(&PayClause<'_>) -> bool,
    read: fn(&PayClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("pay-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("any-amount-of-energy"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_any_amount_of_energy(input)),
    },
    Reading {
        id: RuleId::new("any-amount-of-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_any_amount_of_life(input)),
    },
    Reading {
        id: RuleId::new("one-or-more-energy"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_one_or_more_energy(input)),
    },
    Reading {
        id: RuleId::new("one-or-more-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_one_or_more_life(input)),
    },
    Reading {
        id: RuleId::new("compound-pay"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_pay(input)),
    },
    Reading {
        id: RuleId::new("repeated-tagged-mana-payment"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_tagged_mana_payment(input)),
    },
    Reading {
        id: RuleId::new("mana-for-each-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("repeated-tagged-mana-payment")
        },
        read: |input| input.outcome(read_mana_for_each_count(input)),
    },
    Reading {
        id: RuleId::new("mana-symbol-for-each"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_mana_symbol_for_each(input)),
    },
    Reading {
        id: RuleId::new("half-life-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_half_life_value(input)),
    },
    Reading {
        id: RuleId::new("life-amount"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("mana-symbol-for-each")
        },
        read: |input| input.outcome(read_life_amount(input)),
    },
    Reading {
        id: RuleId::new("energy-amount"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_energy_amount(input)),
    },
    Reading {
        id: RuleId::new("energy-count-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_energy_count_clause(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &PayClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
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
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_any_amount_of_energy(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let energy_symbol_count = input.energy_symbol_count;
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(Some(EffectAst::subject_verb_pay_any_energy(player, 0)));
    }
    Ok(None)
}
fn read_any_amount_of_life(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(Some(EffectAst::subject_verb_pay_any_life(player, 0)));
    }
    Ok(None)
}
fn read_one_or_more_energy(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let energy_symbol_count = input.energy_symbol_count;
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(Some(EffectAst::subject_verb_pay_any_energy(player, 1)));
    }
    Ok(None)
}
fn read_one_or_more_life(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(Some(EffectAst::subject_verb_pay_any_life(player, 1)));
    }
    Ok(None)
}
fn read_compound_pay(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some(compound) = parse_compound_pay(tokens, player) {
        return Ok(Some(compound));
    }
    Ok(None)
}
fn read_repeated_tagged_mana_payment(
    input: &PayClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    // In a clause such as "that player may choose ... and pay {2} for
    // each creature chosen this way", the omitted subject of the payment
    // is the iterated player, not the resolving ability's controller.
    if let Some(repeated) = misc_action_shapes::parse_repeated_tagged_mana_payment_tokens(tokens) {
        let payer = if player == PlayerAst::Implicit {
            PlayerAst::That
        } else {
            player
        };
        return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            effects: vec![EffectAst::subject_verb_pay_mana(
                payer,
                ManaCost::from_pips(repeated.pip_groups),
            )],
        })));
    }
    Ok(None)
}
fn read_mana_for_each_count(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some((for_each_idx, (), _)) =
        grammar::find_prefix(tokens, || grammar::phrase(&["for", "each"]))
        && let Some(parsed_cost) = parse_leaf_mana_cost_prefix_tokens(&tokens[..for_each_idx])
        && parsed_cost.consumed == for_each_idx
        && let [pip] = parsed_cost.cost.pips()
        && let [crate::mana::ManaSymbol::Generic(multiplier)] = pip.as_slice()
    {
        let count_words = crate::lexer::token_word_refs(&tokens[for_each_idx..]);
        if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
        {
            let count = match *multiplier {
                1 => count,
                multiplier => Value::Scaled(Box::new(count), i32::from(multiplier)),
            }
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
            return Ok(Some(subject_verb_player_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                    cost: ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X]),
                    x_value: Some(count),
                    x_maximum: None,
                }),
            )));
        }
    }
    Ok(None)
}
fn read_mana_symbol_for_each(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let clause_words = input.clause_words;
    if clause_words.len() >= 4
        && grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(Some(EffectAst::subject_verb_pay_mana(
            player,
            ManaCost::from_pips(vec![symbols]),
        )));
    }
    Ok(None)
}
fn read_half_life_value(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some(amount) =
        crate::effect_sentences::verb_handlers::parse_half_life_value(tokens, player)
    {
        return Ok(Some(EffectAst::subject_verb_pay_life(player, amount)));
    }
    Ok(None)
}
fn read_life_amount(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some((amount, used)) = parse_value(tokens)
        && token_slice_at_is(tokens, used, "life")
    {
        return Ok(Some(EffectAst::subject_verb_pay_life(player, amount)));
    }
    Ok(None)
}
fn read_energy_amount(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
    {
        return Ok(Some(EffectAst::subject_verb_pay_energy(player, amount)));
    }
    Ok(None)
}
fn read_energy_count_clause(input: &PayClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let energy_symbol_count = input.energy_symbol_count;
    if energy_symbol_count > 0 {
        if let Some((equal_idx, _, _)) =
            grammar::find_prefix(tokens, || grammar::phrase(&["equal", "to"]))
        {
            let amount_tokens = &tokens[equal_idx + 2..];
            if let Some((amount, used)) = parse_value(amount_tokens)
                && used == amount_tokens.len()
            {
                return Ok(Some(EffectAst::subject_verb_pay_energy(player, amount)));
            }
            if let Some(amount) = parse_dynamic_cost_modifier_value(amount_tokens)? {
                return Ok(Some(EffectAst::subject_verb_pay_energy(player, amount)));
            }
        }
        let mut energy_count = 0u32;
        for token in tokens {
            if energy_symbol_token(token) {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word) || misc_word_is_any(word, ENERGY_COUNTER_PAY_IGNORED_WORDS) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            )))
            .map(Some);
        }
        if energy_count > 0 {
            return Ok(Some(EffectAst::subject_verb_pay_energy(
                player,
                Value::Fixed(energy_count as i32),
            )));
        }
    }
    Ok(None)
}
