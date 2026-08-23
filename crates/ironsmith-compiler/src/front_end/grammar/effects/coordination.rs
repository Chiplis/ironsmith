use crate::diagnostics::TextSpan;
use crate::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use crate::model::{
    CarriedFactAst, CoordinationAst, CoordinationBoundaryAst, CoordinationCarryAst,
    CoordinationKindAst, CoordinationMemberAst, CoordinationOperatorAst, EffectDependencyAst,
    EffectOrderingAst,
};
use crate::recognition::{ParseDiagnostic, ParseExpectation, ParseOutcome, RuleId};

use super::chain_splitting::{ChainVerbKind, find_chain_verb_tokens, preserve_and_reason};
use super::typed_clause_heads::{
    ClauseActorHeadAst, ClauseHeadFormAst, TypedClauseHeadAst, classify_typed_clause_head,
};

const COORDINATION_RULE: RuleId = RuleId::new("typed-effect-coordination");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationOmissionAst {
    None,
    Subject,
    Action,
    Object,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecognizedCoordinationBoundary {
    pub operator: CoordinationOperatorAst,
    pub ordering: EffectOrderingAst,
    pub omission: CoordinationOmissionAst,
    pub span: Option<TextSpan>,
}

#[derive(Debug, Clone, Copy)]
pub struct RecognizedCoordinationMember<'a> {
    pub tokens: &'a [OwnedLexToken],
    pub head: Option<TypedClauseHeadAst<'a>>,
    pub span: Option<TextSpan>,
}

#[derive(Debug, Clone)]
pub struct CoordinationPlan<'a> {
    pub kind: CoordinationKindAst,
    pub members: Vec<RecognizedCoordinationMember<'a>>,
    pub boundaries: Vec<RecognizedCoordinationBoundary>,
}

impl CoordinationPlan<'_> {
    /// Build the compiler coordination node once each recognized source
    /// member has produced one semantic effect.  Multi-effect legacy members
    /// remain unwrapped until their parser returns an explicit nested program.
    pub fn into_ast(
        self,
        effects: Vec<crate::cards::builders::EffectAst>,
    ) -> Option<CoordinationAst> {
        if effects.len() != self.members.len() {
            return None;
        }
        let members = effects
            .into_iter()
            .map(|effect| CoordinationMemberAst::new(vec![effect]))
            .collect();
        let boundaries = self
            .boundaries
            .into_iter()
            .enumerate()
            .map(|(index, boundary)| {
                let carries = carry_facts(boundary.omission)
                    .into_iter()
                    .map(|fact| CoordinationCarryAst {
                        from_member: index,
                        to_member: index + 1,
                        fact,
                    })
                    .collect::<Vec<_>>();
                let dependency =
                    if carries.is_empty() && boundary.ordering != EffectOrderingAst::Ordered {
                        EffectDependencyAst::Independent
                    } else {
                        EffectDependencyAst::DependsOnMembers(vec![index])
                    };
                CoordinationBoundaryAst {
                    operator: boundary.operator,
                    ordering: boundary.ordering,
                    dependency,
                    carries,
                    provenance: None,
                }
            })
            .collect();
        CoordinationAst::new(self.kind, members, boundaries, None).ok()
    }

    /// Materialize omitted grammar only for the legacy effect-clause parser.
    /// The returned tokens are not semantic state: `boundaries` remains the
    /// authoritative carry program and is retained in `CoordinationAst`.
    pub fn materialized_segments(&self) -> Option<Vec<Vec<OwnedLexToken>>> {
        let mut materialized = Vec::with_capacity(self.members.len());
        for (member_index, member) in self.members.iter().enumerate() {
            let Some(boundary) = member_index
                .checked_sub(1)
                .and_then(|index| self.boundaries.get(index))
            else {
                materialized.push(member.tokens.to_vec());
                continue;
            };
            let previous = materialized.last()?;
            let tokens = match boundary.omission {
                CoordinationOmissionAst::None | CoordinationOmissionAst::Reference => {
                    member.tokens.to_vec()
                }
                CoordinationOmissionAst::Subject => {
                    let verb_index = find_chain_verb_tokens(previous)?.word_index;
                    if verb_index == 0 {
                        return None;
                    }
                    let subject_prefix = trim_lexed_commas(&previous[..verb_index]);
                    let previous_subject =
                        super::chain_carry::parse_carry_duration_prefix_tokens(subject_prefix)
                            .map_or(subject_prefix, |shape| shape.rest);
                    let mut tokens = carried_subject_surface(previous_subject);
                    tokens.extend(member.tokens.iter().cloned());
                    tokens
                }
                CoordinationOmissionAst::Action | CoordinationOmissionAst::Object => {
                    let verb = find_chain_verb_tokens(previous)?;
                    if !matches!(
                        verb.kind,
                        ChainVerbKind::Deal
                            | ChainVerbKind::Sacrifice
                            | ChainVerbKind::Exile
                            | ChainVerbKind::Create
                    ) {
                        return None;
                    }
                    let mut tokens = previous[..=verb.word_index].to_vec();
                    tokens.extend(member.tokens.iter().cloned());
                    tokens
                }
            };
            materialized.push(tokens);
        }
        Some(materialized)
    }

    /// Return the authored member slices without applying subject carry. This
    /// is used by outer constructs that already own and inject the subject of
    /// every member, such as quantified-participant clauses.
    pub fn member_segments(&self) -> Vec<Vec<OwnedLexToken>> {
        self.members
            .iter()
            .map(|member| member.tokens.to_vec())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinationClauseFacts {
    pub head: super::chain_carry::CarryClauseHead,
    pub imperative_collection_move: bool,
    pub imperative_return: bool,
    pub explicitly_conjugated_player_action: bool,
    pub anaphoric_library_owner: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinationReferenceFacts {
    pub life_stat_pronoun: bool,
    pub affected_object_controller_reward: bool,
    pub implicit_draw_discard_actor: bool,
}

pub fn recognize_coordination_reference_facts(
    tokens: &[OwnedLexToken],
) -> CoordinationReferenceFacts {
    let words = crate::lexer::parser_token_word_refs(tokens);
    const AFFECTED_OBJECT_CONTROLLER_REWARD: &[&str] = &[
        "the",
        "controller",
        "of",
        "each",
        "of",
        "those",
        "artifacts",
        "gains",
        "life",
        "equal",
        "to",
        "its",
        "mana",
        "value",
    ];
    let life_stat_pronoun = words
        .windows(2)
        .any(|words| words[0] == "its" && matches!(words[1], "power" | "toughness"));
    let implicit_draw_discard_actor = words
        .iter()
        .position(|word| matches!(*word, "draw" | "draws"))
        .is_some_and(|draw| {
            words[draw + 1..]
                .iter()
                .any(|word| matches!(*word, "discard" | "discards"))
        });
    CoordinationReferenceFacts {
        life_stat_pronoun,
        affected_object_controller_reward: words.ends_with(AFFECTED_OBJECT_CONTROLLER_REWARD),
        implicit_draw_discard_actor,
    }
}

pub fn recognize_coordination_clause_facts(tokens: &[OwnedLexToken]) -> CoordinationClauseFacts {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let significant = tokens
        .iter()
        .filter(|token| !token.parser_word_pieces().is_empty())
        .skip_while(|token| token.is_word("then") || token.is_word("and"))
        .collect::<Vec<_>>();
    let first = significant.first().copied();
    CoordinationClauseFacts {
        head: super::chain_carry::parse_carry_clause_head_tokens(tokens),
        imperative_collection_move: first.is_some_and(|token| token.is_word("put")),
        imperative_return: first.is_some_and(|token| token.is_word("return")),
        explicitly_conjugated_player_action: first.is_some_and(|token| {
            token.slice.eq_ignore_ascii_case("draws")
                || token.slice.eq_ignore_ascii_case("scries")
                || token.slice.eq_ignore_ascii_case("surveils")
        }),
        anaphoric_library_owner: words
            .windows(2)
            .any(|window| window == ["their", "library"])
            || words
                .windows(4)
                .any(|window| window == ["his", "or", "her", "library"]),
    }
}

/// Materialize a structurally omitted subject for the compatibility clause
/// parser.  The decision is made from typed verb positions and the dedicated
/// carryable-subject grammar before the follow-up effect is parsed.
pub fn materialize_shared_subject_followup(
    previous: &[OwnedLexToken],
    followup: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let followup_verb = find_chain_verb_tokens(followup)?;
    if followup_verb.word_index != 0
        || !matches!(
            followup_verb.kind,
            ChainVerbKind::Gain | ChainVerbKind::Lose | ChainVerbKind::Become
        )
    {
        return None;
    }
    let previous_verb = find_chain_verb_tokens(previous)?;
    if previous_verb.word_index == 0 {
        return None;
    }
    let subject_prefix = trim_lexed_commas(&previous[..previous_verb.word_index]);
    let subject = super::chain_carry::parse_carry_duration_prefix_tokens(subject_prefix)
        .map_or(subject_prefix, |shape| shape.rest);
    if subject.is_empty() || super::chain_carry::parse_carryable_subject_tokens(subject).is_none() {
        return None;
    }
    let mut materialized = carried_subject_surface(subject);
    materialized.extend(followup.iter().cloned());
    Some(materialized)
}

pub fn recognize_coordination(tokens: &[OwnedLexToken]) -> ParseOutcome<CoordinationPlan<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        trim_lexed_commas(&tokens[1..])
    } else {
        tokens
    };
    if tokens.is_empty() {
        return ParseOutcome::NoMatch;
    }
    let candidates = top_level_boundaries(tokens);
    let mut members = Vec::new();
    let mut boundaries = Vec::new();
    let mut member_start = 0usize;

    for candidate in candidates {
        let before = trim_lexed_commas(&tokens[member_start..candidate.start]);
        let after = trim_lexed_commas(&tokens[candidate.end..]);
        if before.is_empty() || after.is_empty() {
            if candidate.commits_without_head() {
                return malformed_boundary(candidate.span, "effect clause after connective");
            }
            continue;
        }
        let Some((boundary, next_head)) = classify_boundary(candidate, before, after) else {
            continue;
        };
        let member_tokens = trim_lexed_commas(&tokens[member_start..candidate.start]);
        let member_head = matched_head(member_tokens);
        members.push(RecognizedCoordinationMember {
            tokens: member_tokens,
            head: member_head,
            span: token_span(member_tokens),
        });
        boundaries.push(boundary);
        member_start = candidate.end;

        // `next_head` is deliberately computed at the boundary, where a
        // missing head means grammatical carry.  The final member stores the
        // same classification below without rescanning registries.
        let _ = next_head;
    }

    if boundaries.is_empty() {
        return ParseOutcome::NoMatch;
    }
    let tail = trim_lexed_commas(&tokens[member_start..]);
    if tail.is_empty() {
        return malformed_boundary(token_span(tokens), "final coordinated effect clause");
    }
    members.push(RecognizedCoordinationMember {
        tokens: tail,
        head: matched_head(tail),
        span: token_span(tail),
    });
    if members.len() != boundaries.len() + 1 {
        return ParseOutcome::Error(ParseDiagnostic::invariant(
            COORDINATION_RULE,
            token_span(tokens),
            "coordination member and boundary counts diverged",
        ));
    }
    let kind = coordination_kind(&boundaries);
    ParseOutcome::matched(
        CoordinationPlan {
            kind,
            members,
            boundaries,
        },
        token_span(tokens),
    )
}

#[derive(Debug, Clone, Copy)]
struct BoundaryCandidate {
    start: usize,
    end: usize,
    operator: CoordinationOperatorAst,
    span: Option<TextSpan>,
}

impl BoundaryCandidate {
    fn commits_without_head(self) -> bool {
        matches!(
            self.operator,
            CoordinationOperatorAst::Then
                | CoordinationOperatorAst::CommaThen
                | CoordinationOperatorAst::Semicolon
        )
    }
}

fn top_level_boundaries(tokens: &[OwnedLexToken]) -> Vec<BoundaryCandidate> {
    let mut boundaries = Vec::new();
    let mut quote_open = false;
    let mut parenthesis_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        let token = &tokens[index];
        match token.kind {
            TokenKind::Quote => quote_open = !quote_open,
            TokenKind::LParen if !quote_open => parenthesis_depth += 1,
            TokenKind::RParen if !quote_open => {
                parenthesis_depth = parenthesis_depth.saturating_sub(1)
            }
            TokenKind::LBracket if !quote_open => bracket_depth += 1,
            TokenKind::RBracket if !quote_open => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
        if quote_open || parenthesis_depth != 0 || bracket_depth != 0 {
            index += 1;
            continue;
        }
        let (end, operator) = if token.kind == TokenKind::Comma
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("then"))
        {
            (index + 2, CoordinationOperatorAst::CommaThen)
        } else if token.kind == TokenKind::Semicolon {
            (index + 1, CoordinationOperatorAst::Semicolon)
        } else if token.is_word("and") {
            (index + 1, CoordinationOperatorAst::And)
        } else if token.is_word("or") {
            (index + 1, CoordinationOperatorAst::Or)
        } else if token.is_word("then") {
            (index + 1, CoordinationOperatorAst::Then)
        } else if token.kind == TokenKind::Comma {
            (index + 1, CoordinationOperatorAst::Comma)
        } else {
            index += 1;
            continue;
        };
        boundaries.push(BoundaryCandidate {
            start: index,
            end,
            operator,
            span: token_span(&tokens[index..end]),
        });
        index = end;
    }
    boundaries
}

fn classify_boundary<'a>(
    candidate: BoundaryCandidate,
    before: &[OwnedLexToken],
    after: &'a [OwnedLexToken],
) -> Option<(
    RecognizedCoordinationBoundary,
    Option<TypedClauseHeadAst<'a>>,
)> {
    if candidate.operator == CoordinationOperatorAst::Comma
        && super::for_each_shapes::parse_participant_clause_shape(before)
            .is_some_and(|shape| !shape.participant_is_actor && shape.inner_tokens.is_empty())
    {
        // `For each player/opponent, <program>` owns this delimiter as part
        // of the quantifier. Splitting it as effect coordination creates an
        // empty loop followed by an unrelated top-level action.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::And
        && preserve_and_reason(before, after, true).is_some()
    {
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && before
            .iter()
            .any(|token| token.is_word("choose") || token.is_word("chooses"))
        && after
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| {
                word == "nontoken" || word == "non-token" || word.starts_with("non-")
            })
        && crate::object_filters::parse_object_filter(after, false).is_ok()
    {
        // A serial negative modifier is still part of the chosen object
        // filter: `choose two nontoken, non-Vehicle creatures ...`.  The
        // relative `they control` tail contains a verb, so generic
        // coordination otherwise mistakes the adjective comma for a new
        // ordered action and emits a verb-less `non-Vehicle creatures ...`
        // clause.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Or
        && before.last().is_some_and(token_is_card_type_noun)
        && after.first().is_some_and(token_is_card_type_noun)
    {
        // A card-type union is one object operand even when a later action
        // follows in the same sentence. Do not let the typed clause-head
        // classifier reinterpret the second type as a structural action
        // head (for example, "sacrifices a creature or planeswalker ... and
        // loses 1 life").
        return None;
    }
    let before_verb = find_chain_verb_tokens(before);
    let after_verb = find_chain_verb_tokens(after);
    let after_head = matched_head(after);

    let omission = match after_head {
        Some(TypedClauseHeadAst {
            actor: ClauseActorHeadAst::Implicit,
            form: ClauseHeadFormAst::Action(_),
            ..
        }) if before_verb.is_some() => CoordinationOmissionAst::Subject,
        Some(TypedClauseHeadAst {
            actor: ClauseActorHeadAst::Reference,
            ..
        }) => CoordinationOmissionAst::Reference,
        Some(_) => CoordinationOmissionAst::None,
        None if before_verb.is_some()
            && after_verb.is_none()
            && (candidate.operator != CoordinationOperatorAst::Or
                || after.first().is_some_and(|token| {
                    token.is_word("target")
                        || token.is_word("it")
                        || token.is_word("that")
                        || token.is_word("those")
                })) =>
        {
            CoordinationOmissionAst::Action
        }
        None => return None,
    };

    if matches!(candidate.operator, CoordinationOperatorAst::Comma)
        && matches!(omission, CoordinationOmissionAst::Action)
    {
        return None;
    }
    if matches!(
        candidate.operator,
        CoordinationOperatorAst::And | CoordinationOperatorAst::Or
    ) && omission == CoordinationOmissionAst::None
        && before_verb.is_none()
    {
        return None;
    }
    let ordering = match candidate.operator {
        CoordinationOperatorAst::Or => EffectOrderingAst::Alternative,
        CoordinationOperatorAst::And => EffectOrderingAst::Unordered,
        CoordinationOperatorAst::Then
        | CoordinationOperatorAst::Comma
        | CoordinationOperatorAst::CommaThen
        | CoordinationOperatorAst::Semicolon
        | CoordinationOperatorAst::SentenceBoundary => EffectOrderingAst::Ordered,
    };
    Some((
        RecognizedCoordinationBoundary {
            operator: candidate.operator,
            ordering,
            omission,
            span: candidate.span,
        },
        after_head,
    ))
}

fn token_is_card_type_noun(token: &OwnedLexToken) -> bool {
    token.as_word().is_some_and(|word| {
        matches!(
            word,
            "artifact"
                | "battle"
                | "creature"
                | "enchantment"
                | "instant"
                | "land"
                | "planeswalker"
                | "sorcery"
        )
    })
}

fn coordination_kind(boundaries: &[RecognizedCoordinationBoundary]) -> CoordinationKindAst {
    let first = boundary_kind(boundaries[0]);
    if boundaries
        .iter()
        .copied()
        .all(|boundary| boundary_kind(boundary) == first)
    {
        first
    } else {
        CoordinationKindAst::Mixed
    }
}

fn boundary_kind(boundary: RecognizedCoordinationBoundary) -> CoordinationKindAst {
    match boundary.omission {
        CoordinationOmissionAst::Subject => CoordinationKindAst::SharedSubject,
        CoordinationOmissionAst::Action | CoordinationOmissionAst::Object => {
            CoordinationKindAst::SharedObject
        }
        CoordinationOmissionAst::Reference => CoordinationKindAst::Carry,
        CoordinationOmissionAst::None => match boundary.ordering {
            EffectOrderingAst::Ordered => CoordinationKindAst::Sequence,
            EffectOrderingAst::Unordered => CoordinationKindAst::Conjunction,
            EffectOrderingAst::Alternative => CoordinationKindAst::Disjunction,
        },
    }
}

fn carry_facts(omission: CoordinationOmissionAst) -> Vec<CarriedFactAst> {
    match omission {
        CoordinationOmissionAst::None => Vec::new(),
        CoordinationOmissionAst::Subject => vec![CarriedFactAst::Subject(None)],
        CoordinationOmissionAst::Action => vec![CarriedFactAst::Action(None)],
        CoordinationOmissionAst::Object => vec![CarriedFactAst::Object(None)],
        CoordinationOmissionAst::Reference => vec![CarriedFactAst::Reference(None)],
    }
}

fn carried_subject_surface(subject: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words = subject
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if matches!(
        words.as_slice(),
        ["target", "player"] | ["target", "opponent"]
    ) {
        vec![
            OwnedLexToken::synthetic_word("that"),
            OwnedLexToken::synthetic_word("player"),
        ]
    } else if words.first() == Some(&"target") {
        vec![OwnedLexToken::synthetic_word("it")]
    } else {
        subject.to_vec()
    }
}

fn matched_head(tokens: &[OwnedLexToken]) -> Option<TypedClauseHeadAst<'_>> {
    match classify_typed_clause_head(tokens) {
        ParseOutcome::Match(matched) => Some(matched.value),
        ParseOutcome::NoMatch | ParseOutcome::Error(_) => None,
    }
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

fn malformed_boundary<T>(span: Option<TextSpan>, expected: &'static str) -> ParseOutcome<T> {
    ParseOutcome::Error(ParseDiagnostic::malformed(
        COORDINATION_RULE,
        span,
        [ParseExpectation::new(expected)],
        "authored coordination boundary has no complete following clause",
    ))
}
