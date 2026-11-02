//! The readings of one granted ability component ("gains <ability>"): a
//! top-level activated or triggered ability, a quoted object restriction,
//! quoted static abilities, keyword actions, an equip line, a static ability
//! line, a single keyword word. Formerly a first-match ladder in
//! `gain_ability`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct GrantedComponent<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) clause_words: &'a [&'a str],
    pub(super) authored_as_quoted_ability: bool,
    pub(super) top_level_activated_ability: bool,
    pub(super) top_level_triggered_ability: bool,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl GrantedComponent<'_> {
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
                crate::word_claims::probe_only(|| {
                    (reading.admits)(self)
                        && matches!((reading.read)(self), ParseOutcome::Match(_))
                })
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<GrantedAbilityAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<GrantedAbilityAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("granted-ability-component-registry-reading"),
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
    admits: fn(&GrantedComponent<'_>) -> bool,
    read: fn(&GrantedComponent<'_>) -> ParseOutcome<Vec<GrantedAbilityAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("granted-ability-component-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("top-level-activated-or-triggered"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_top_level_activated_or_triggered(input)),
    },
    Reading {
        id: RuleId::new("direct-quoted-object-restriction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("top-level-activated-or-triggered")
        },
        read: |input| input.outcome(read_direct_quoted_object_restriction(input)),
    },
    Reading {
        id: RuleId::new("quoted-static-abilities"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("direct-quoted-object-restriction")
                && !input.read_by("top-level-activated-or-triggered")
        },
        read: |input| input.outcome(read_quoted_static_abilities(input)),
    },
    Reading {
        id: RuleId::new("activated-or-triggered"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activated_or_triggered(input)),
    },
    Reading {
        id: RuleId::new("keyword-actions"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("direct-quoted-object-restriction")
        },
        read: |input| input.outcome(read_keyword_actions(input)),
    },
    Reading {
        id: RuleId::new("equip-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_equip_line(input)),
    },
    Reading {
        id: RuleId::new("static-ability-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("activated-or-triggered")
                && !input.read_by("direct-quoted-object-restriction")
                && !input.read_by("keyword-actions")
        },
        read: |input| input.outcome(read_static_ability_line(input)),
    },
    Reading {
        id: RuleId::new("single-word-keyword"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_single_word_keyword(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(
    input: &GrantedComponent<'_>,
) -> ParseOutcome<RuleMatch<Vec<GrantedAbilityAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !crate::word_claims::probe_only(|| (reading.admits)(input)) {
            continue;
        }
        let (registry_outcome, registry_claims) =
            crate::word_claims::scoped(|| (reading.read)(input).within(reading.id));
        match registry_outcome {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            ).with_claims(registry_claims)),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<Vec<GrantedAbilityAst>>> = Vec::new();
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

fn read_top_level_activated_or_triggered(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let clause_words = input.clause_words;
    let top_level_activated_ability = input.top_level_activated_ability;
    let top_level_triggered_ability = input.top_level_triggered_ability;
    let ability_tokens = input.tokens;
    if (top_level_activated_ability || top_level_triggered_ability)
        && let Some(granted) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![granted]));
    }
    Ok(None)
}
fn read_direct_quoted_object_restriction(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    // This complete intrinsic rule owns its named payment operand. The
    // general negated-object reader is prefix-tolerant and cannot own the
    // trailing qualifier. Token callers may have removed the outer quotes.
    if let Some(ability) = crate::consumer_mana::named_tap_prohibition_ability(input.tokens) {
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(ability.into()))]));
    }
    let authored_as_quoted_ability = input.authored_as_quoted_ability;
    let ability_tokens = input.tokens;
    if (authored_as_quoted_ability
        || crate::lexer::token_word_refs(ability_tokens).starts_with(&[
            "its",
            "activated",
            "abilities",
        ]))
        && let Some(restriction) = parse_direct_quoted_object_restriction(&ability_tokens)?
    {
        return Ok(Some(restriction));
    }
    Ok(None)
}
fn read_quoted_static_abilities(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let authored_as_quoted_ability = input.authored_as_quoted_ability;
    let ability_tokens = input.tokens;
    if authored_as_quoted_ability
        && let Some(facts) = crate::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(ability_tokens)
            .statement.as_enters_effect_program
        && !facts.turns_face_up_only
        && ability_tokens.first().is_some_and(|token| token.is_word("as"))
    {
        let Some(comma) = ability_tokens.iter().position(OwnedLexToken::is_comma)
            else { return Ok(None); };
        let mut effects = crate::semantic_line_parsing::parse_effect_sentences_preserving_source_boundaries(
            &ability_tokens[comma + 1..])?;
        if effects.is_empty() { return Ok(None); }
        if facts.source_reference_enters_with_counter_surface {
            ironsmith_compiler_resolve::reference_resolution::resolve_as_enters_source_counter_grants(&mut effects);
        }
        crate::word_claims::claim_tokens(&ability_tokens[..=comma]);
        let ability = StaticAbility::as_enters_effect_program(
            effects.into(), facts.subject, facts.also_turns_face_up,
            facts.uses_enters_with_counter_surface, None,
        );
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
            StaticAbilityAst::Static(ability),
        ))]));
    }
    if authored_as_quoted_ability
        && let Some(static_abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)?
        && !static_abilities.is_empty()
    {
        if matches!(
            static_abilities.as_slice(),
            [StaticAbilityAst::Static(ability)]
                if ability.id() == StaticAbilityId::Unblockable
        ) {
            let restriction = crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            );
            return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
                StaticAbilityAst::Static(StaticAbility::restriction(
                    restriction,
                    "This creature can't be blocked.".to_string(),
                )),
            ))]));
        }
        return parsed_static_granted_abilities(&ability_tokens, static_abilities).map(Some);
    }
    Ok(None)
}
fn read_activated_or_triggered(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let clause_words = input.clause_words;
    let ability_tokens = input.tokens;
    if let Some(granted) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![granted]));
    }
    Ok(None)
}
fn read_keyword_actions(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let clause_words = input.clause_words;
    let authored_as_quoted_ability = input.authored_as_quoted_ability;
    let ability_tokens = input.tokens;
    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        if authored_as_quoted_ability && matches!(actions.as_slice(), [KeywordAction::Unblockable])
        {
            let restriction = crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            );
            return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
                StaticAbilityAst::Static(StaticAbility::restriction(
                    restriction,
                    "This creature can't be blocked.".to_string(),
                )),
            ))]));
        }
        return Ok(Some(actions.into_iter().map(|action| {
            if !authored_as_quoted_ability
                && let KeywordAction::ProtectionFromColorsAmong(filter) = &action
            {
                return GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
                    StaticAbility::protection(crate::ability::ProtectionFrom::ColorsAmongAtResolution(filter.clone())),
                )));
            }
            GrantedAbilityAst::from(action)
        }).collect()));
    }
    Ok(None)
}
fn read_equip_line(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(equip_spec) =
        crate::grammar::keyword_activated_lines::parse_equip_line_spec_tokens(&ability_tokens)
        && let Some(parsed) =
            crate::activation_and_restrictions::parse_equip_line_lexed(&ability_tokens)?
    {
        let typed_cost = match &equip_spec {
            crate::grammar::keyword_activated_lines::EquipLineSpec::Mana { cost } => {
                cost.to_oracle()
            }
            crate::grammar::keyword_activated_lines::EquipLineSpec::QualifiedCost {
                qualifier,
                ..
            } => {
                let qualifier = qualifier
                    .subtypes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let crate::model::CompilerAbilityKindCore::Activated(activated) = parsed.kind()
                else {
                    return Err(CardTextError::InvariantViolation(
                        "equip grammar produced a non-activated ability".to_string(),
                    ));
                };
                format!("{qualifier} {}", activated.mana_cost.display())
            }
            crate::grammar::keyword_activated_lines::EquipLineSpec::ActivationCost { .. } => {
                let crate::model::CompilerAbilityKindCore::Activated(activated) = parsed.kind()
                else {
                    return Err(CardTextError::InvariantViolation(
                        "equip grammar produced a non-activated ability".to_string(),
                    ));
                };
                activated.mana_cost.display()
            }
            crate::grammar::keyword_activated_lines::EquipLineSpec::MissingCost => {
                return Err(CardTextError::InvariantViolation(
                    "equip grammar produced an ability without a cost".to_string(),
                ));
            }
        };
        return Ok(Some(vec![GrantedAbilityAst::ParsedObjectAbility {
            display: format!("Equip {typed_cost}"),
            ability: Box::new(parsed),
        }]));
    }
    Ok(None)
}
fn read_static_ability_line(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(parsed_static_granted_abilities(
            &ability_tokens,
            abilities,
        )?));
    }
    Ok(None)
}
fn read_single_word_keyword(
    input: &GrantedComponent<'_>,
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(action) = ability_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .filter(|_| ability_tokens.len() == 1)
        .and_then(parse_single_word_keyword_action)
    {
        return Ok(Some(vec![GrantedAbilityAst::from(action)]));
    }
    Ok(None)
}

#[cfg(test)]
mod complete_quoted_entry_program_tests {
    use super::*;
    #[test]
    fn quoted_entry_program_retains_both_local_producer_and_source_counter_instruction() {
        for amount in ["2", "two"] {
            let tokens = crate::lexer::lex_line(&format!("\"As this permanent enters, you may sacrifice up to {amount} artifacts and/or creatures. This permanent enters with that many +1/+1 counters on it.\""), 0).unwrap();
            let (abilities, choice) = super::super::parse_granted_abilities_for_gain_clause(
                &tokens, &crate::lexer::token_word_refs(&tokens), false).unwrap();
            assert!(!choice);
            let [GrantedAbilityAst::StaticAbility(ability)] = abilities.as_slice() else { panic!("one complete entry program: {abilities:?}"); };
            let StaticAbilityAst::Static(ability) = ability.as_ref() else { panic!("typed static entry"); };
            let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {program,..} = &ability.payload else { panic!("entry owner"); };
            fn inspect(
                effect: &crate::cards::builders::EffectAst,
                choices: &mut Vec<(crate::cards::builders::TagKey, crate::ChoiceCount)>,
                sacrifices: &mut Vec<ObjectFilter>,
                counters: &mut Vec<(crate::cards::builders::TargetAst, crate::effect::Value)>,
            ) {
                use crate::cards::builders::{EffectAst, SubjectVerbActionAst, CounterActionAst, ZoneMoveActionAst, ObjectChoiceEffectAst};
                if let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {count,tag,..}) = effect {
                    choices.push((tag.key.clone(), *count));
                }
                if let EffectAst::SubjectVerb(subject) = effect {
                    if let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll {filter}) = &subject.action {
                        sacrifices.push(filter.clone());
                    }
                    if let SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {target,count,..}) = &subject.action {
                        counters.push((target.clone(), count.clone()));
                    }
                }
                crate::model::visit::for_each_nested_effects(effect, true, |children| {
                    for child in children { inspect(child, choices, sacrifices, counters); }
                });
            }
            let mut choices = Vec::new(); let mut sacrifices = Vec::new(); let mut counters = Vec::new();
            for effect in program.all_effects() { inspect(effect, &mut choices, &mut sacrifices, &mut counters); }
            let [(selected, count)] = choices.as_slice() else { panic!("one bounded selection: {choices:?}"); };
            assert_eq!(count.min, 0); assert_eq!(count.max, Some(2));
            assert_eq!(sacrifices, vec![ObjectFilter::tagged(selected.clone())],
                "one retained producer sacrifices exactly its selected set");
            let [(target, count)] = counters.as_slice() else { panic!("one entrant counter action: {counters:?}"); };
            assert!(matches!(target, crate::cards::builders::TargetAst::Source(_)));
            assert!(!matches!(count, crate::effect::Value::Fixed(_)),
                "counter count retains its local producer dependency");
        }
    }
}

#[cfg(test)]
mod named_payment_component_tests {
    use super::*;
    #[test]
    fn exact_source_payment_rule_precedes_prefix_restriction_with_or_without_outer_quote_role() {
        for quoted in [false, true] {
            let tokens=crate::lexer::lex_line("This creature can't be tapped to pay the generic tap payment named cadence craft.",0).unwrap();
            let words=crate::lexer::token_word_refs(&tokens);
            let input=GrantedComponent { tokens:&tokens,clause_words:&words,authored_as_quoted_ability:quoted,
                top_level_activated_ability:false,top_level_triggered_ability:false,read_by_cache:Default::default() };
            let abilities=read_direct_quoted_object_restriction(&input).unwrap().unwrap();
            assert_eq!(abilities.len(),1);
            let GrantedAbilityAst::StaticAbility(ability)=&abilities[0] else{panic!("expected static grant")};
            let StaticAbilityAst::Static(ability)=ability.as_ref() else{panic!("expected intrinsic source rule")};
            assert!(matches!(&ability.payload,ironsmith_core::StaticAbilityPayload::GenericManaTapPaymentProhibition{name} if name=="cadence craft"));
        }
    }
}
