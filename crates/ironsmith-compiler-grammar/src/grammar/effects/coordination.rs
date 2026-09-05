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
        mut effects: Vec<crate::cards::builders::EffectAst>,
    ) -> Option<CoordinationAst> {
        if effects.len() != self.members.len() {
            return None;
        }
        for (boundary_index, boundary) in self.boundaries.iter().enumerate() {
            if boundary.omission == CoordinationOmissionAst::Reference
                && member_produces_plural_created_collection(&effects[boundary_index])
            {
                bind_singular_damage_source_to_ability_source(&mut effects[boundary_index + 1]);
            }
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
            let member_tokens = trim_lexed_commas(member.tokens);
            let member_tokens = if member_tokens
                .first()
                .is_some_and(|token| token.is_word("and") || token.is_word("then"))
            {
                trim_lexed_commas(&member_tokens[1..])
            } else {
                member_tokens
            };
            let Some(boundary) = member_index
                .checked_sub(1)
                .and_then(|index| self.boundaries.get(index))
            else {
                materialized.push(member_tokens.to_vec());
                continue;
            };
            let previous = materialized.last()?;
            let tokens = match boundary.omission {
                CoordinationOmissionAst::None | CoordinationOmissionAst::Reference => {
                    member_tokens.to_vec()
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
                    tokens.extend(member_tokens.iter().cloned());
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
                    tokens.extend(member_tokens.iter().cloned());
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

/// Build a canonical coordination node for a grammar rule that has already
/// proved the relationship between its parsed members.
pub fn coordination_from_effects(
    kind: CoordinationKindAst,
    operator: CoordinationOperatorAst,
    ordering: EffectOrderingAst,
    effects: Vec<crate::cards::builders::EffectAst>,
) -> Option<CoordinationAst> {
    if effects.len() < 2 {
        return None;
    }
    let members = effects
        .into_iter()
        .map(|effect| CoordinationMemberAst::new(vec![effect]))
        .collect::<Vec<_>>();
    let boundaries = (1..members.len())
        .map(|to_member| CoordinationBoundaryAst {
            operator,
            ordering,
            dependency: if ordering == EffectOrderingAst::Ordered {
                EffectDependencyAst::DependsOnMembers(vec![to_member - 1])
            } else {
                EffectDependencyAst::Independent
            },
            carries: Vec::new(),
            provenance: None,
        })
        .collect::<Vec<_>>();
    CoordinationAst::new(kind, members, boundaries, None).ok()
}

fn member_produces_plural_created_collection(effect: &crate::cards::builders::EffectAst) -> bool {
    use crate::cards::builders::{EffectAst, SubjectVerbActionAst};
    use crate::effect::Value;

    if matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                SubjectVerbActionAst::CreateTokenWithMods { count, .. }
                    if !matches!(count.unhinted(), Value::Fixed(1))
            )
    ) {
        return true;
    }
    let mut found = false;
    crate::model::visit::for_each_nested_effects(effect, true, |nested| {
        found |= nested.iter().any(member_produces_plural_created_collection);
    });
    found
}

fn bind_singular_damage_source_to_ability_source(effect: &mut crate::cards::builders::EffectAst) {
    use crate::cards::builders::{EffectAst, SubjectVerbActionAst, TagKey, TargetAst};
    use crate::target::ObjectFilter;

    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::DealDamageEqualToPower { source, .. } =
            &mut subject_verb.action
    {
        match source {
            TargetAst::Tagged(tag, span)
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
            {
                *source = TargetAst::Source(*span);
                return;
            }
            TargetAst::Object(filter, None, span)
                if *filter == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()) =>
            {
                *source = TargetAst::Source(*span);
                return;
            }
            _ => {}
        }
    }
    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            bind_singular_damage_source_to_ability_source(nested_effect);
        }
    });
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
    let life_stat_pronoun = words.iter().enumerate().any(|(index, word)| {
        *word == "its"
            && words
                .get(index + 1)
                .is_some_and(|stat| matches!(*stat, "power" | "toughness"))
    });
    let implicit_draw_discard_actor =
        crate::slice_primitives::select_position(&words, |word| matches!(*word, "draw" | "draws"))
            .is_some_and(|draw| {
                words[draw + 1..]
                    .iter()
                    .any(|word| matches!(*word, "discard" | "discards"))
            });
    CoordinationReferenceFacts {
        life_stat_pronoun,
        affected_object_controller_reward: crate::word_primitives::parse_sequence_suffix(
            &words,
            AFFECTED_OBJECT_CONTROLLER_REWARD,
        ),
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
        anaphoric_library_owner: crate::word_primitives::any_sequence_occurs(
            &words,
            &[&["their", "library"], &["his", "or", "her", "library"]],
        ),
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
    if tokens
        .first()
        .is_some_and(|token| token.is_word("if") || token.is_word("unless"))
    {
        // Leading control flow owns the complete consequence program. Its
        // body may itself contain coordination, but splitting the outer line
        // first would leave later consequence members outside the condition.
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
        } else if token.kind == TokenKind::Comma
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("and"))
        {
            (index + 2, CoordinationOperatorAst::And)
        } else if token.kind == TokenKind::Comma
            && tokens.get(index + 1).is_some_and(|next| next.is_word("or"))
        {
            (index + 2, CoordinationOperatorAst::Or)
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
    if boundary_continues_shuffle_zone_list(candidate.operator, before, after) {
        // "shuffles their hand and graveyard into their library" is one
        // shuffle whose object is a zone union; the connective is not an
        // action boundary.
        return None;
    }
    if boundary_continues_filter_keyword_list(candidate.operator, before, after) {
        // A comma or connective between keyword constraints belongs to the
        // surrounding object predicate or ability list. In particular,
        // `doesn't have first strike, double strike, vigilance, or haste`
        // must remain one filter; `haste` is a keyword constraint, not an
        // executable alternative effect.
        return None;
    }
    if matches!(
        candidate.operator,
        CoordinationOperatorAst::Comma | CoordinationOperatorAst::And
    ) && super::chain_splitting::is_creature_subtype_subject_list_boundary(before, after)
    {
        // Serial subtype subjects are one filter even though the final arm
        // contains the clause's eventual verb: `Birds, Frogs, Otters, and
        // Rats you control get ...`. Neither the commas nor the final `and`
        // introduce executable effect members.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && before.first().is_some_and(|token| token.is_word("if"))
        && !before.iter().any(|token| token.kind == TokenKind::Comma)
    {
        // The first comma in a leading-if sentence terminates the predicate.
        // It is owned by control-flow grammar even when the consequence later
        // contains its own comma/and coordination.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && super::chain_splitting::is_token_creation_context_tokens(before)
        && before
            .iter()
            .any(|token| token.is_word("copy") || token.is_word("copies"))
        && after.first().is_some_and(|token| token.is_word("except"))
        && after.get(1).is_some_and(|token| {
            token.is_word("it") || token.is_word("its") || token.is_word("their")
        })
    {
        // A copy-token exception modifies the preceding creation action. It
        // is not a second coordinated effect merely because characteristic
        // prose inside the exception contains finite `is` verbs.
        return None;
    }
    if matches!(
        candidate.operator,
        CoordinationOperatorAst::Comma | CoordinationOperatorAst::And
    ) && before.last().is_some_and(token_is_card_type_noun)
        && starts_card_type_list_arm(after)
        && (find_chain_verb_tokens(before).is_some_and(|verb| {
            matches!(
                verb.kind,
                ChainVerbKind::Destroy | ChainVerbKind::Exile | ChainVerbKind::Sacrifice
            )
        }) || before
            .iter()
            .any(|token| token.is_word("choose") || token.is_word("chooses")))
    {
        // Serial card-type domains remain one operand of the action even
        // when an arm carries a qualifier: `all artifacts, enchantments, and
        // nonbasic lands`. A card-type word such as `land` can also be a verb,
        // so this boundary must be rejected before clause-head inference.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && comma_continues_for_each_object_filter(before, after)
    {
        // A serial object domain belongs to the dynamic count operand, not
        // to effect coordination: `create ... for each tapped Assassin,
        // Pirate, and/or Vehicle you control`. Both commas must stay inside
        // the filter so the creation parser receives the complete union.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && comma_continues_excluded_literal_name(before, after)
    {
        // Literal card names may contain commas. Prove that the complete
        // target through the following `onto` destination is a valid object
        // filter carrying an excluded name before treating the comma as an
        // executable effect boundary.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && before.iter().enumerate().any(|(index, _)| {
            let duration = super::chain_carry::parse_carry_duration_prefix_tokens(&before[index..]);
            duration.is_some_and(|shape| {
                shape.rest.is_empty()
                    && (index == 0
                        || before[index - 1].is_word("and")
                        || before[index - 1].is_word("then"))
            })
        })
    {
        // This comma closes an authored duration introducer, not an effect
        // member: `[action], and until your next turn, [scoped actions]`.
        // Keeping it inside the member lets the chain parser carry that
        // duration across the remaining coordinated clauses.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && super::for_each_shapes::parse_participant_clause_shape(before)
            .is_some_and(|shape| !shape.participant_is_actor && shape.inner_tokens.is_empty())
    {
        // `For each player/opponent, <program>` owns this delimiter as part
        // of the quantifier. Splitting it as effect coordination creates an
        // empty loop followed by an unrelated top-level action.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Comma {
        let before_words = crate::lexer::parser_token_word_refs(before);
        if crate::word_primitives::parse_any_sequence_complete(
            &before_words,
            &[
                &["after", "this", "phase"],
                &["after", "this", "main", "phase"],
            ],
        ) {
            // The comma terminates the timing introducer of an additional-
            // phase instruction; it does not separate two effect members.
            return None;
        }
    }
    if candidate.operator == CoordinationOperatorAst::And
        && preserve_and_reason(before, after, true).is_some()
    {
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Or {
        let before_words = crate::lexer::parser_token_word_refs(before);
        let after_words = crate::lexer::parser_token_word_refs(after);
        if or_continues_explicit_target_domain(&before_words, &after_words) {
            return None;
        }
        if crate::word_primitives::sequence_occurs(&before_words, &["protection", "from"])
            && crate::word_primitives::parse_sequence_prefix(&after_words, &["from"])
        {
            // Protection domains may coordinate two `from` operands inside
            // one granted ability: `protection from colorless or from the
            // color of your choice`. The second prepositional phrase is not
            // an alternative executable effect.
            return None;
        }
        if crate::word_primitives::parse_sequence_suffix(
            &before_words,
            &["from", "your", "graveyard"],
        ) && crate::word_primitives::parse_any_sequence_prefix(
            &after_words,
            &[&["from", "exile"], &["exile"]],
        ) {
            // A source-return origin may name the two zones as one domain:
            // `return this card from your graveyard or from exile ...`.
            // The second origin has no executable head; keep the union inside
            // the return clause so its dedicated lowering can retain both
            // functional zones and emit the combined runtime action.
            return None;
        }
        if (crate::word_primitives::parse_sequence_suffix(&before_words, &["hand"])
            && crate::word_primitives::parse_sequence_prefix(&after_words, &["graveyard"]))
            || (crate::word_primitives::parse_sequence_suffix(&before_words, &["graveyard"])
                && crate::word_primitives::parse_sequence_prefix(&after_words, &["hand"]))
        {
            // A hand-or-graveyard pair is one zone domain of the surrounding
            // object selection. A later executable action can make the
            // second zone arm look like a structural clause head, but the
            // `or` still belongs to the target rather than effect
            // coordination.
            return None;
        }
        if after_words
            .first()
            .is_some_and(|word| matches!(*word, "less" | "greater" | "more" | "fewer"))
        {
            // `3 or less`, `X or greater`, and their count variants are one
            // comparison operand. The comparative adjective is not an
            // alternative executable member of the surrounding effect.
            return None;
        }
        if crate::word_primitives::parse_any_sequence_suffix(
            &before_words,
            &[&["less", "than"], &["greater", "than"]],
        ) && crate::word_primitives::parse_sequence_prefix(&after_words, &["equal", "to"])
        {
            // `less/greater than or equal to` is one comparison operator,
            // not a disjunction between executable effect members.
            return None;
        }
    }
    if candidate.operator == CoordinationOperatorAst::Comma
        && before
            .iter()
            .any(|token| token.is_word("choose") || token.is_word("chooses"))
        && after
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| {
                word == "nontoken"
                    || word == "non-token"
                    || crate::word_primitives::parse_word_prefix(word, "non-")
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
    if candidate.operator == CoordinationOperatorAst::And
        && before.last().is_some_and(token_is_card_type_noun)
        && after.first().is_some_and(token_is_card_type_noun)
        && find_chain_verb_tokens(after).is_none()
    {
        // A conjunctive card-type list is likewise one object operand when
        // the later type arm contains no executable verb. Relative filter
        // nouns such as `counters on it` must remain inside that operand.
        return None;
    }
    let before_words = crate::lexer::parser_token_word_refs(before);
    let after_words = crate::lexer::parser_token_word_refs(after);
    if candidate.operator == CoordinationOperatorAst::Or
        && crate::word_primitives::sequence_occurs(&before_words, &["target"])
        && before_words
            .last()
            .is_some_and(|word| matches!(*word, "legendary" | "basic" | "snow"))
        && after_words
            .iter()
            .take_while(|word| !matches!(**word, "and" | "then"))
            .any(|word| matches!(*word, "card" | "cards" | "permanent" | "permanents"))
    {
        // A supertype-or-type/subtype union is one target domain:
        // `target legendary or Rat card`. The terminal card noun scopes the
        // second arm, while the first arm is intentionally abbreviated.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::Or
        && ((crate::word_primitives::parse_sequence_suffix(&before_words, &["target", "player"])
            && crate::word_primitives::parse_sequence_prefix(&after_words, &["planeswalker"]))
            || ((crate::word_primitives::parse_sequence_suffix(
                &before_words,
                &["that", "player"],
            ) || crate::word_primitives::parse_sequence_suffix(
                &before_words,
                &["that", "opponent"],
            )) && crate::word_primitives::parse_any_sequence_prefix(
                &after_words,
                &[
                    &["planeswalker"],
                    &["planeswalkers"],
                    &["that", "planeswalker"],
                    &["that", "planeswalkers"],
                ],
            )))
    {
        // Player-or-planeswalker is one target/controller operand. The
        // second noun is not an alternative executable action.
        return None;
    }
    if candidate.operator == CoordinationOperatorAst::And
        && before_words
            .iter()
            .any(|word| matches!(*word, "deal" | "deals"))
        && crate::word_primitives::sequence_occurs(&before_words, &["damage"])
        && crate::word_primitives::parse_sequence_prefix(&after_words, &["each", "creature"])
        && crate::word_primitives::sequence_occurs(&after_words, &["controls"])
    {
        // The damage fanout owns both its primary target and the creatures
        // controlled by that target/controller. Keep the correlated object
        // union intact for the typed damage recognizer.
        return None;
    }
    let before_verb = find_chain_verb_tokens(before);
    let after_verb = find_chain_verb_tokens(after);
    let after_head = matched_head(after);
    let starts_shared_object_operand = after.first().is_some_and(|token| {
        token.is_word("a")
            || token.is_word("an")
            || token.is_word("all")
            || token.is_word("another")
            || token.is_word("it")
            || token.is_word("target")
            || token.is_word("that")
            || token.is_word("the")
            || token.is_word("them")
            || token.is_word("these")
            || token.is_word("this")
            || token.is_word("those")
            || token.is_word("up")
    });
    let before_starts_shared_object_operand = before.first().is_some_and(|token| {
        token.is_word("a")
            || token.is_word("an")
            || token.is_word("all")
            || token.is_word("another")
            || token.is_word("it")
            || token.is_word("target")
            || token.is_word("that")
            || token.is_word("the")
            || token.is_word("them")
            || token.is_word("these")
            || token.is_word("this")
            || token.is_word("those")
            || token.is_word("up")
    });

    let omission = match after_head {
        Some(_)
            if matches!(
                candidate.operator,
                CoordinationOperatorAst::Comma | CoordinationOperatorAst::And
            ) && before_verb.is_none()
                && after_verb.is_none()
                && before_starts_shared_object_operand
                && starts_shared_object_operand =>
        {
            // In a serial operand list, later comma members inherit the
            // action already materialized for the first member. Authored
            // slices after the first comma intentionally contain no verb.
            CoordinationOmissionAst::Action
        }
        Some(TypedClauseHeadAst { form, .. })
            if before_verb.is_some()
                && after_verb.is_none()
                && starts_shared_object_operand
                && matches!(
                    form,
                    ClauseHeadFormAst::Structural
                        | ClauseHeadFormAst::Action(crate::model::ClauseVerbAst::Counter)
                        | ClauseHeadFormAst::Action(crate::model::ClauseVerbAst::Control)
                        | ClauseHeadFormAst::Action(crate::model::ClauseVerbAst::DealDamage)
                ) =>
        {
            CoordinationOmissionAst::Action
        }
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

fn boundary_continues_shuffle_zone_list(
    operator: CoordinationOperatorAst,
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
) -> bool {
    if !matches!(operator, CoordinationOperatorAst::And) {
        return false;
    }
    let after_words = crate::lexer::parser_token_word_refs(after);
    if !matches!(
        after_words.first(),
        Some(&"hand" | &"hands" | &"graveyard" | &"graveyards" | &"library" | &"libraries")
    ) || !after_words.iter().any(|word| *word == "into")
    {
        return false;
    }
    let before_words = crate::lexer::parser_token_word_refs(before);
    // Only a zone-to-zone union ("their hand and graveyard into ...") stays
    // one shuffle object. An object-plus-zone union ("this artifact and your
    // graveyard") keeps its coordination boundary so the authored arms can be
    // rejoined by the renderer.
    before_words
        .iter()
        .any(|word| matches!(*word, "shuffle" | "shuffles"))
        && before_words.iter().any(|word| {
            matches!(
                *word,
                "hand" | "hands" | "graveyard" | "graveyards" | "library" | "libraries"
            )
        })
}

fn boundary_continues_filter_keyword_list(
    operator: CoordinationOperatorAst,
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
) -> bool {
    let after_words = crate::lexer::parser_token_word_refs(after);
    let continues_keyword_list =
        if crate::util::starts_filter_keyword_list_continuation_words(&after_words) {
            true
        } else {
            let connector = match operator {
                CoordinationOperatorAst::And => "and",
                CoordinationOperatorAst::Or => "or",
                _ => return false,
            };
            let mut continuation_words = Vec::with_capacity(after_words.len() + 1);
            continuation_words.push(connector);
            continuation_words.extend(after_words);
            crate::util::starts_filter_keyword_list_continuation_words(&continuation_words)
        };
    if !continues_keyword_list {
        return false;
    }
    // The keyword continuation belongs to the surrounding filter only when
    // the nearest governing head before the boundary is a possession or
    // filter head. An effect verb such as `gains`/`loses` makes the keyword
    // arms executable alternatives that coordination must keep separate.
    let before_words = crate::lexer::parser_token_word_refs(before);
    for word in before_words.iter().rev() {
        match *word {
            "have" | "has" | "had" | "with" | "without" => return true,
            "gain" | "gains" | "gained" | "lose" | "loses" | "lost" => return false,
            _ => {}
        }
    }
    true
}

fn token_is_card_type_noun(token: &OwnedLexToken) -> bool {
    token.parser_word_pieces().iter().any(|piece| {
        matches!(
            piece.text.as_str(),
            "artifact"
                | "artifacts"
                | "nonartifact"
                | "nonartifacts"
                | "battle"
                | "battles"
                | "nonbattle"
                | "nonbattles"
                | "creature"
                | "creatures"
                | "noncreature"
                | "noncreatures"
                | "enchantment"
                | "enchantments"
                | "nonenchantment"
                | "nonenchantments"
                | "instant"
                | "instants"
                | "noninstant"
                | "noninstants"
                | "land"
                | "lands"
                | "nonland"
                | "nonlands"
                | "planeswalker"
                | "planeswalkers"
                | "nonplaneswalker"
                | "nonplaneswalkers"
                | "sorcery"
                | "sorceries"
                | "nonsorcery"
                | "nonsorceries"
        )
    })
}

fn starts_card_type_list_arm(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    let tokens = if tokens.first().is_some_and(|token| {
        token.is_word("and")
            || token.is_word("or")
            || token.parser_text().eq_ignore_ascii_case("and/or")
    }) {
        &tokens[1..]
    } else {
        tokens
    };
    let tokens = if tokens.first().is_some_and(|token| {
        token.is_word("basic")
            || token.is_word("nonbasic")
            || token.is_word("token")
            || token.is_word("nontoken")
    }) {
        &tokens[1..]
    } else {
        tokens
    };
    tokens.first().is_some_and(token_is_card_type_noun)
}

fn or_continues_explicit_target_domain(before_words: &[&str], after_words: &[&str]) -> bool {
    let before_ends_with =
        |suffix: &[&str]| crate::word_primitives::parse_sequence_suffix(before_words, suffix);
    let after_starts_with =
        |prefix: &[&str]| crate::word_primitives::parse_sequence_prefix(after_words, prefix);

    // Player/permanent target unions remain one legality domain even when a
    // later explicit action gives the second noun a plausible clause head.
    ((before_ends_with(&["target", "player"])
        || before_ends_with(&["target", "opponent"]))
        && (after_starts_with(&["planeswalker"])
            || after_starts_with(&["battle"])))
        || ((before_ends_with(&["target", "planeswalker"])
            || before_ends_with(&["target", "battle"]))
            && (after_starts_with(&["player"])
                || after_starts_with(&["opponent"])))
        // A shared terminal creature noun scopes both combat-role
        // adjectives. Splitting here both invents a modal choice and drops
        // one half of the target's legal combat-state domain.
        || (before_ends_with(&["target", "attacking"])
            && after_starts_with(&["blocking", "creature"]))
        || (before_ends_with(&["target", "blocking"])
            && after_starts_with(&["attacking", "creature"]))
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
    let words = crate::lexer::parser_token_word_refs(subject);
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[&["target", "player"], &["target", "opponent"]],
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

fn comma_continues_for_each_object_filter(
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
) -> bool {
    let Some(for_each_start) = before
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            (token.is_word("for")
                && before
                    .get(index + 1)
                    .is_some_and(|next| next.is_word("each")))
            .then_some(index)
        })
        .next_back()
    else {
        return false;
    };
    let filter_prefix = trim_lexed_commas(&before[for_each_start + 2..]);
    if filter_prefix.is_empty() || after.is_empty() {
        return false;
    }

    let after_starts_final_arm = after.first().is_some_and(|token| {
        token.is_word("and")
            || token.is_word("or")
            || token.parser_text().eq_ignore_ascii_case("and/or")
    });
    let after_contains_later_serial_boundary = after.iter().enumerate().any(|(index, token)| {
        token.kind == TokenKind::Comma
            && after.get(index + 1).is_some_and(|next| {
                next.is_word("and")
                    || next.is_word("or")
                    || next.parser_text().eq_ignore_ascii_case("and/or")
            })
    });
    if !after_starts_final_arm && !after_contains_later_serial_boundary {
        return false;
    }

    let mut filter_tokens = filter_prefix.to_vec();
    filter_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    filter_tokens.extend_from_slice(after);
    crate::object_filters::parse_object_filter(&filter_tokens, false).is_ok()
}

fn comma_continues_excluded_literal_name(
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
) -> bool {
    let Some(put_index) = crate::slice_primitives::select_last_position(before, |token| {
        token.is_word("put") || token.is_word("puts")
    }) else {
        return false;
    };
    let target_prefix = &before[put_index + 1..];
    let prefix_words = crate::lexer::parser_token_word_refs(target_prefix);
    if !crate::word_primitives::sequence_occurs(&prefix_words, &["not", "named"]) {
        return false;
    }
    let Some(onto_index) =
        crate::slice_primitives::select_position(after, |token| token.is_word("onto"))
    else {
        return false;
    };
    let mut target_tokens = target_prefix.to_vec();
    target_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    target_tokens.extend_from_slice(&after[..onto_index]);
    crate::object_filters::parse_object_filter(&target_tokens, false)
        .is_ok_and(|filter| filter.excluded_name.is_some())
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
