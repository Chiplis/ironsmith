//! The readings of one "create ..." clause before the token-definition
//! grammar: a choice of options, a direct alternative, a direct conjunction,
//! a delayed combat token action. Formerly a first-match ladder in
//! `creation_handlers`; every reading runs, resolved by rank while the
//! overlaps are measured. The token definition is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CreateClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) subject: Option<SubjectAst>,
}

impl CreateClause<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("create-clause-registry-reading"),
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
    admits: fn(&CreateClause<'_>) -> bool,
    read: fn(&CreateClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("create-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("lexical-token-prototype-reference"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_token_prototype_reference(input)),
    },
    Reading {
        id: RuleId::new("choice-of-options"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choice_of_options(input)),
    },
    Reading {
        id: RuleId::new("direct-token-creation-alternative"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_token_creation_alternative(input)),
    },
    Reading {
        id: RuleId::new("direct-token-creation-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_token_creation_conjunction(input)),
    },
    Reading {
        id: RuleId::new("delayed-combat-token-action"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delayed_combat_token_action(input)),
    },
];

/// Count and entry modifiers belong to this instruction; the complete token
/// definition is bound later, across the containing ability's lexical scope.
fn read_token_prototype_reference(input: &CreateClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let Some(head) = creation_grammar::parse_create_head_tokens(input.tokens) else {
        return Ok(None);
    };
    if head.name_words != ["of", "those"] {
        return Ok(None);
    }
    let invalid = || CardTextError::ParseError(
        "unsupported token prototype reference count or modifier".into(),
    );
    let (mut count, tail) = creation_grammar::parse_token_prototype_reference_head(input.tokens)
        .ok_or_else(invalid)?;
    let tail = if tail.last().is_some_and(|token| token.kind == TokenKind::Period) {
        &tail[..tail.len() - 1]
    } else { tail };
    let (entry, binding) = if let Some(where_at) = tail.iter().position(|t| t.is_word("where")) {
        let entry = &tail[..where_at];
        let entry = if entry.last().is_some_and(|token| token.kind == TokenKind::Comma) {
            &entry[..entry.len() - 1]
        } else { entry };
        (entry, Some(&tail[where_at..]))
    } else {
        (tail, None)
    };
    let attacking = if entry.is_empty() { false } else if
        crate::grammar::primitives::probe_all(
            entry, crate::grammar::primitives::phrase(&["that", "are", "tapped", "and", "attacking"]),
            "token prototype entry modifier",
        ).is_some() { true } else { return Err(invalid()); };
    if let Some(binding) = binding {
        if !value_contains_unbound_x(&count) {
            return Err(invalid());
        }
        count = with_where_x_surface_hints(
            parse_create_value_binding(binding)?.ok_or_else(invalid)?, input.tokens,
        );
    }
    let player = extract_subject_player(input.subject).unwrap_or(PlayerAst::Implicit);
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        player,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            name: "those tokens".into(),
            definition: crate::model::token_definition::TokenDefinitionSpec::PrototypeReference(
                crate::model::token_definition::TokenPrototypeReference::PreviousDefinition,
            ),
            count,
            dynamic_power_toughness: None,
            player,
            actor_surface_explicit: matches!(input.subject, Some(SubjectAst::Player(PlayerAst::You))),
            attached_to: None,
            tapped: attacking,
            attacking,
            attack_target_player: None,
            combat_entry: Default::default(),
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        }),
    )))
}

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CreateClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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

fn read_choice_of_options(input: &CreateClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(choice) = parse_create_choice_of_options(tokens)? {
        return Ok(Some(choice));
    }
    Ok(None)
}
fn read_direct_token_creation_alternative(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    if let Some(alternative) = parse_direct_token_creation_alternative(tokens, subject) {
        return Ok(Some(alternative));
    }
    Ok(None)
}
fn read_direct_token_creation_conjunction(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    if let Some(conjunction) = parse_direct_token_creation_conjunction(tokens, subject) {
        return Ok(Some(conjunction));
    }
    Ok(None)
}
fn read_delayed_combat_token_action(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let non_article_words = crate::util::non_article_token_word_refs(tokens);
    if let Some(action) =
        creation_grammar::parse_delayed_combat_token_action_words(&non_article_words)
    {
        let effect = match action {
            creation_grammar::DelayedCombatTokenAction::Exile => EffectAst::subject_verb_exile(
                TargetAst::Object(
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    span_from_tokens(tokens),
                    None,
                ),
                false,
            ),
            creation_grammar::DelayedCombatTokenAction::Sacrifice => {
                EffectAst::subject_verb_sacrifice(
                    PlayerAst::Implicit,
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    1,
                    None,
                )
            }
        };
        return Ok(Some(EffectAst::Delayed(
            DelayedEffectAst::DelayedUntilEndOfCombat {
                effects: vec![effect],
            },
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod prototype_reference_tests {
    use super::*;

    #[test]
    fn prototype_reference_retains_count_binding_and_combat_override() {
        for (text, attacking) in [
            ("Create two of those tokens", false),
            ("Create two of those tokens that are tapped and attacking", true),
            ("Create X of those tokens, where X is the number of creature cards in your graveyard", false),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let parsed = parse_create(&tokens, None).unwrap();
            let EffectAst::SubjectVerb(subject) = parsed else { panic!("{parsed:?}") };
            let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                definition, count, tapped, attacking: actual_attacking, ..
            }) = subject.action else { panic!("expected typed prototype reference") };
            assert!(matches!(definition,
                crate::model::token_definition::TokenDefinitionSpec::PrototypeReference(_)));
            assert_eq!(actual_attacking, attacking);
            assert_eq!(tapped, attacking);
            assert!(!matches!(count, Value::X), "where-X binding must remain executable");
        }
    }

    #[test]
    fn malformed_prototype_references_are_not_shortened_into_supported_actions() {
        for text in [
            "Create of those tokens",
            "Create two of those token",
            "Create two; of those tokens",
            "Create two of, those tokens",
            "Create two of those tokens that are tapped, and attacking",
            "Create two of those tokens and gibberish",
            "Create two of those tokens, where X is the number of creatures you control",
            "Create two of those tokens {G}",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse_create(&tokens, None).is_err(), "{text}");
        }
    }

    #[test]
    fn exact_adipose_body_binds_the_winning_where_x_reader_to_paid_emerge_evidence() {
        let tokens = crate::lexer::lex_line(
            "Create a 2/2 white Alien creature token. If this creature's emerge cost was paid, instead create X of those tokens, where X is the sacrificed creature's toughness.", 0).unwrap();
        let parsed = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        let normalized = ironsmith_compiler_resolve::effect_ast_normalization::normalize_effects_ast(&parsed);
        let prepared = ironsmith_compiler_resolve::reference_resolution::annotate_effect_sequence(
            &normalized, &Default::default(), Default::default(), Default::default()).unwrap();
        fn check(effects: &[EffectAst], found: &mut usize) {
            for effect in effects {
                if let EffectAst::SubjectVerb(subject) = effect
                    && let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }) = &subject.action
                    && let Value::ToughnessOf(reference) = count.unhinted()
                {
                    assert!(matches!(reference.base(), crate::target::ChooseSpec::Tagged(tag)
                        if tag.as_str() == ironsmith_core::tag::SOURCE_EMERGE_SACRIFICE_TAG));
                    *found += 1;
                }
                crate::model::visit::for_each_nested_effects(effect, true, |nested| check(nested, found));
            }
        }
        let mut found = 0;
        for annotated in &prepared.effects { check(std::slice::from_ref(&annotated.effect), &mut found); }
        assert_eq!(found, 1);
    }
}
