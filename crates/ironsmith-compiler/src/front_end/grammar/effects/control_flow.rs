use crate::diagnostics::TextSpan;
use crate::grammar::structure::{
    parse_if_result_predicate, parse_predicate_with_grammar_entrypoint_lexed,
    split_trailing_if_clause_lexed,
};
use crate::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use crate::model::control_flow::{PermissionRelationshipAst, PreventionRelationshipAst};
use crate::model::{ClauseActorAst, ClauseVerbAst};
use crate::model::{
    CompilerControlFlowAst, CompilerDurationAst, ConditionPositionAst, ControlConditionAst,
    ControlFlowNodeAst, ControlFlowSemanticAst, ControlPredicateAst, DelayedScheduleAst,
    NestedProgramAst, NestedProgramKindAst, ReplacedEventAst, ReplacementKindAst,
    ReplacementRelationshipAst,
};
use crate::recognition::{ParseDiagnostic, ParseExpectation, ParseOutcome, RuleId};

use super::typed_clause_heads::{
    ClauseActorHeadAst, ClauseHeadFormAst, classify_typed_clause_head,
};

const CONTROL_FLOW_RULE: RuleId = RuleId::new("typed-effect-control-flow");

#[derive(Debug, Clone)]
pub enum RecognizedControlFlowAst {
    Condition {
        condition: ControlConditionAst,
        reflexive: bool,
    },
    Replacement {
        kind: ReplacementKindAst,
        event: ReplacedEventAst,
        condition: Option<ControlConditionAst>,
    },
    Duration(CompilerDurationAst),
    Delayed {
        schedule: DelayedScheduleAst,
        duration: Option<CompilerDurationAst>,
        reflexive: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ControlFlowPlan<'a> {
    pub structure: RecognizedControlFlowAst,
    pub body_tokens: &'a [OwnedLexToken],
    pub parse_original_with_legacy: bool,
    pub span: Option<TextSpan>,
}

impl ControlFlowPlan<'_> {
    pub fn into_ast(
        self,
        effects: Vec<crate::cards::builders::EffectAst>,
    ) -> Option<CompilerControlFlowAst> {
        if effects.is_empty() {
            return None;
        }
        let effects = wrap_body_semantics(self.body_tokens, effects);
        let body_kind = match &self.structure {
            RecognizedControlFlowAst::Replacement { .. } => NestedProgramKindAst::Replacement,
            RecognizedControlFlowAst::Delayed {
                reflexive: true, ..
            } => NestedProgramKindAst::Reflexive,
            RecognizedControlFlowAst::Delayed { .. } => NestedProgramKindAst::Delayed,
            RecognizedControlFlowAst::Condition { .. } | RecognizedControlFlowAst::Duration(_) => {
                NestedProgramKindAst::Consequence
            }
        };
        let programs = vec![NestedProgramAst::new(body_kind, effects)];
        let (semantic, node) = match self.structure {
            RecognizedControlFlowAst::Condition {
                condition,
                reflexive,
            } => (
                ControlFlowSemanticAst::ControlFlow,
                ControlFlowNodeAst::Condition {
                    condition,
                    consequence_program: 0,
                    alternative_program: None,
                    reflexive,
                },
            ),
            RecognizedControlFlowAst::Replacement {
                kind,
                event,
                condition,
            } => (
                ControlFlowSemanticAst::Replacement,
                ControlFlowNodeAst::Replacement(ReplacementRelationshipAst {
                    kind,
                    event,
                    condition,
                    original_program: None,
                    replacement_program: 0,
                    affected_reference: None,
                }),
            ),
            RecognizedControlFlowAst::Duration(duration) => (
                ControlFlowSemanticAst::ControlFlow,
                ControlFlowNodeAst::Duration {
                    duration,
                    program: 0,
                },
            ),
            RecognizedControlFlowAst::Delayed {
                schedule,
                duration,
                reflexive,
            } => (
                ControlFlowSemanticAst::ControlFlow,
                ControlFlowNodeAst::Delayed {
                    schedule,
                    duration,
                    program: 0,
                    one_shot: true,
                    reflexive,
                    watched_references: Vec::new(),
                },
            ),
        };
        CompilerControlFlowAst::new(semantic, node, programs, None).ok()
    }
}

pub fn recognize_control_flow(tokens: &[OwnedLexToken]) -> ParseOutcome<ControlFlowPlan<'_>> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return ParseOutcome::NoMatch;
    }
    if let Some(plan) = recognize_leading_replacement(tokens) {
        return ParseOutcome::matched(plan, token_span(tokens));
    }
    if let Some(plan) = recognize_delayed_or_duration(tokens) {
        return ParseOutcome::matched(plan, token_span(tokens));
    }
    if let Some(plan) = recognize_leading_condition(tokens) {
        return plan;
    }
    if let Some(plan) = recognize_trailing_condition(tokens) {
        return plan;
    }
    ParseOutcome::NoMatch
}

fn recognize_leading_replacement(tokens: &[OwnedLexToken]) -> Option<ControlFlowPlan<'_>> {
    if tokens.first().is_some_and(|token| token.is_word("instead")) {
        let body_tokens = trim_lexed_commas(&tokens[1..]);
        if body_tokens.is_empty() {
            return None;
        }
        return Some(ControlFlowPlan {
            structure: RecognizedControlFlowAst::Replacement {
                kind: ReplacementKindAst::Instead,
                event: ReplacedEventAst::PriorEffect,
                condition: None,
            },
            body_tokens,
            parse_original_with_legacy: false,
            span: token_span(tokens),
        });
    }
    if tokens.first().is_some_and(|token| token.is_word("as")) {
        let comma = first_top_level_comma(tokens)?;
        let condition_tokens = trim_lexed_commas(&tokens[1..comma]);
        let body_tokens = trim_lexed_commas(&tokens[comma + 1..]);
        if condition_tokens.is_empty() || body_tokens.is_empty() {
            return None;
        }
        let condition =
            parse_state_condition(condition_tokens, ConditionPositionAst::Precondition, false)?;
        return Some(ControlFlowPlan {
            structure: RecognizedControlFlowAst::Replacement {
                kind: ReplacementKindAst::As,
                event: ReplacedEventAst::EnterBattlefield,
                condition: Some(condition),
            },
            body_tokens,
            parse_original_with_legacy: false,
            span: token_span(tokens),
        });
    }
    None
}

fn recognize_delayed_or_duration(tokens: &[OwnedLexToken]) -> Option<ControlFlowPlan<'_>> {
    const DELAYED_PREFIXES: &[(&[&str], DelayedScheduleAst)] = &[
        (
            &["at", "the", "beginning", "of", "the", "next", "end", "step"],
            DelayedScheduleAst::NextEndStep,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "cleanup",
                "step",
            ],
            DelayedScheduleAst::NextCleanupStep,
        ),
        (
            &["at", "the", "beginning", "of", "your", "next", "upkeep"],
            DelayedScheduleAst::NextUpkeep,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "draw",
                "step",
            ],
            DelayedScheduleAst::NextDrawStep,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "main",
                "phase",
            ],
            DelayedScheduleAst::NextMainPhase,
        ),
        (
            &["at", "end", "of", "combat"],
            DelayedScheduleAst::EndOfCombat,
        ),
    ];
    for (prefix, schedule) in DELAYED_PREFIXES {
        if let Some(body_tokens) = body_after_prefix_and_comma(tokens, prefix) {
            return Some(ControlFlowPlan {
                structure: RecognizedControlFlowAst::Delayed {
                    schedule: *schedule,
                    duration: None,
                    reflexive: false,
                },
                // Existing delayed parsers already materialize scheduling;
                // parse the original once with control recognition disabled.
                body_tokens,
                parse_original_with_legacy: true,
                span: token_span(tokens),
            });
        }
    }

    const FIXED_DURATIONS: &[(&[&str], CompilerDurationAst)] = &[
        (
            &["until", "end", "of", "turn"],
            CompilerDurationAst::UntilEndOfTurn,
        ),
        (
            &["until", "end", "of", "combat"],
            CompilerDurationAst::UntilEndOfCombat,
        ),
        (&["this", "turn"], CompilerDurationAst::ThisTurn),
        (
            &["until", "your", "next", "turn"],
            CompilerDurationAst::UntilNextTurn,
        ),
    ];
    for (prefix, duration) in FIXED_DURATIONS {
        if let Some(body_tokens) = body_after_prefix_and_comma(tokens, prefix) {
            return Some(ControlFlowPlan {
                structure: RecognizedControlFlowAst::Duration(duration.clone()),
                body_tokens,
                parse_original_with_legacy: true,
                span: token_span(tokens),
            });
        }
    }

    let for_as_long_as = ["for", "as", "long", "as"];
    if starts_with_words(tokens, &for_as_long_as) {
        let comma = first_top_level_comma(tokens)?;
        let condition_tokens = trim_lexed_commas(&tokens[for_as_long_as.len()..comma]);
        let body_tokens = trim_lexed_commas(&tokens[comma + 1..]);
        if body_tokens.is_empty() {
            return None;
        }
        let predicate = parse_predicate_with_grammar_entrypoint_lexed(condition_tokens).ok()?;
        return Some(ControlFlowPlan {
            structure: RecognizedControlFlowAst::Duration(CompilerDurationAst::ForAsLongAs(
                predicate,
            )),
            body_tokens,
            parse_original_with_legacy: true,
            span: token_span(tokens),
        });
    }
    None
}

fn recognize_leading_condition(
    tokens: &[OwnedLexToken],
) -> Option<ParseOutcome<ControlFlowPlan<'_>>> {
    let introducer = tokens.first()?.as_word()?;
    let (negated_surface, reflexive) = match introducer {
        "if" => (false, false),
        "unless" => (true, false),
        "when" => (false, true),
        _ => return None,
    };
    let Some(comma) = first_top_level_comma(tokens) else {
        return Some(malformed(token_span(tokens), "comma-delimited consequence"));
    };
    let condition_tokens = trim_lexed_commas(&tokens[1..comma]);
    let body_tokens = trim_lexed_commas(&tokens[comma + 1..]);
    if condition_tokens.is_empty() || body_tokens.is_empty() {
        return Some(malformed(token_span(tokens), "condition and consequence"));
    }
    let condition = if let Some(result) = parse_if_result_predicate(condition_tokens) {
        ControlConditionAst {
            position: ConditionPositionAst::ResultCondition,
            predicate: ControlPredicateAst::Result(result),
            negated_surface,
            provenance: None,
        }
    } else if introducer == "when" {
        // A non-result `when` clause is event grammar owned by trigger and
        // delayed-program recognition, not an ordinary state condition.
        return None;
    } else {
        parse_state_condition(
            condition_tokens,
            ConditionPositionAst::Precondition,
            negated_surface,
        )?
    };
    Some(ParseOutcome::matched(
        ControlFlowPlan {
            structure: RecognizedControlFlowAst::Condition {
                condition,
                reflexive,
            },
            body_tokens,
            parse_original_with_legacy: false,
            span: token_span(tokens),
        },
        token_span(tokens),
    ))
}

fn recognize_trailing_condition(
    tokens: &[OwnedLexToken],
) -> Option<ParseOutcome<ControlFlowPlan<'_>>> {
    if let Some(split) = split_trailing_if_clause_lexed(tokens) {
        return Some(ParseOutcome::matched(
            ControlFlowPlan {
                structure: RecognizedControlFlowAst::Condition {
                    condition: ControlConditionAst {
                        position: ConditionPositionAst::Postcondition,
                        predicate: ControlPredicateAst::State(split.predicate),
                        negated_surface: false,
                        provenance: None,
                    },
                    reflexive: false,
                },
                body_tokens: split.leading_tokens,
                parse_original_with_legacy: false,
                span: token_span(tokens),
            },
            token_span(tokens),
        ));
    }
    let unless = last_top_level_word(tokens, "unless")?;
    let body_tokens = trim_lexed_commas(&tokens[..unless]);
    let condition_tokens = trim_lexed_commas(&tokens[unless + 1..]);
    if body_tokens.is_empty() || condition_tokens.is_empty() {
        return Some(malformed(token_span(tokens), "trailing unless condition"));
    }
    let condition =
        parse_state_condition(condition_tokens, ConditionPositionAst::Postcondition, true)?;
    Some(ParseOutcome::matched(
        ControlFlowPlan {
            structure: RecognizedControlFlowAst::Condition {
                condition,
                reflexive: false,
            },
            body_tokens,
            parse_original_with_legacy: false,
            span: token_span(tokens),
        },
        token_span(tokens),
    ))
}

fn wrap_body_semantics(
    tokens: &[OwnedLexToken],
    effects: Vec<crate::cards::builders::EffectAst>,
) -> Vec<crate::cards::builders::EffectAst> {
    let head = match classify_typed_clause_head(tokens) {
        ParseOutcome::Match(matched) => matched.value,
        ParseOutcome::NoMatch | ParseOutcome::Error(_) => return effects,
    };
    let ClauseHeadFormAst::Action(action) = head.form else {
        return effects;
    };
    if action == ClauseVerbAst::Prevent {
        let node = ControlFlowNodeAst::Prevention(PreventionRelationshipAst {
            event: ReplacedEventAst::Damage,
            condition: None,
            prevention_program: 0,
            protected_reference: None,
        });
        let programs = vec![NestedProgramAst::new(
            NestedProgramKindAst::Prevention,
            effects,
        )];
        let control =
            CompilerControlFlowAst::new(ControlFlowSemanticAst::Prevention, node, programs, None)
                .expect("prevention body constructs one valid nested program");
        return vec![crate::cards::builders::EffectAst::ControlFlow(Box::new(
            control,
        ))];
    }
    let contains_permission = tokens
        .iter()
        .any(|token| token.is_word("may") || token.is_word("can") || token.is_word("could"));
    if contains_permission {
        let actor = match head.actor {
            ClauseActorHeadAst::Player if head.first_word == "opponent" => {
                ClauseActorAst::EachOpponent
            }
            ClauseActorHeadAst::Iterated => ClauseActorAst::EachPlayer,
            _ => ClauseActorAst::SourceController,
        };
        let node = ControlFlowNodeAst::Permission(PermissionRelationshipAst {
            actor,
            action,
            duration: None,
            program: 0,
        });
        let programs = vec![NestedProgramAst::new(
            NestedProgramKindAst::Permission,
            effects,
        )];
        let control =
            CompilerControlFlowAst::new(ControlFlowSemanticAst::Permission, node, programs, None)
                .expect("permission body constructs one valid nested program");
        return vec![crate::cards::builders::EffectAst::ControlFlow(Box::new(
            control,
        ))];
    }
    effects
}

fn parse_state_condition(
    tokens: &[OwnedLexToken],
    position: ConditionPositionAst,
    negated_surface: bool,
) -> Option<ControlConditionAst> {
    let predicate = parse_predicate_with_grammar_entrypoint_lexed(tokens).ok()?;
    Some(ControlConditionAst {
        position,
        predicate: ControlPredicateAst::State(predicate),
        negated_surface,
        provenance: None,
    })
}

fn body_after_prefix_and_comma<'a>(
    tokens: &'a [OwnedLexToken],
    prefix: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    if !starts_with_words(tokens, prefix) {
        return None;
    }
    let comma = tokens.get(prefix.len())?;
    if comma.kind != TokenKind::Comma {
        return None;
    }
    let body = trim_lexed_commas(&tokens[prefix.len() + 1..]);
    (!body.is_empty()).then_some(body)
}

fn starts_with_words(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    tokens.len() >= words.len()
        && tokens
            .iter()
            .zip(words)
            .all(|(token, word)| token.is_word(word))
}

fn first_top_level_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut quoted = false;
    let mut parenthesis_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => quoted = !quoted,
            TokenKind::LParen if !quoted => parenthesis_depth += 1,
            TokenKind::RParen if !quoted => parenthesis_depth = parenthesis_depth.saturating_sub(1),
            TokenKind::LBracket if !quoted => bracket_depth += 1,
            TokenKind::RBracket if !quoted => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Comma if !quoted && parenthesis_depth == 0 && bracket_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn last_top_level_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    let mut quoted = false;
    let mut parenthesis_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut found = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => quoted = !quoted,
            TokenKind::LParen if !quoted => parenthesis_depth += 1,
            TokenKind::RParen if !quoted => parenthesis_depth = parenthesis_depth.saturating_sub(1),
            TokenKind::LBracket if !quoted => bracket_depth += 1,
            TokenKind::RBracket if !quoted => bracket_depth = bracket_depth.saturating_sub(1),
            _ if !quoted
                && parenthesis_depth == 0
                && bracket_depth == 0
                && token.is_word(expected) =>
            {
                found = Some(index)
            }
            _ => {}
        }
    }
    found
}

fn token_span(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    (first.span.line == last.span.line).then_some(TextSpan {
        line: first.span.line,
        start: first.span.start,
        end: last.span.end,
    })
}

fn malformed<T>(span: Option<TextSpan>, expected: &'static str) -> ParseOutcome<T> {
    ParseOutcome::Error(ParseDiagnostic::malformed(
        CONTROL_FLOW_RULE,
        span,
        [ParseExpectation::new(expected)],
        "control-flow introducer has no complete scoped program",
    ))
}
