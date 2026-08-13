use winnow::combinator::eof;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::{leaf, permission_shapes, primitives};
use crate::recognition::ParseOutcome;
use crate::runtime_backend::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice,
};

#[path = "document_shapes/labels.rs"]
mod labels;
pub(crate) use labels::*;
#[path = "document_shapes/choice_context.rs"]
mod choice_context;
pub(crate) use choice_context::*;
#[path = "document_shapes/unsupported.rs"]
mod unsupported;
pub(crate) use unsupported::*;
#[path = "document_shape_parsers.rs"]
mod shape_parsers;
use shape_parsers::{
    additional_activation_cost_head, alias_face_separator, source_alias_effect_verb,
    when_one_or_more_followup_head,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerCapSurface {
    Once,
    Twice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerCapSuffixShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) cap: TriggerCapSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NamedOptionChoiceHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonpermanentStatementSurface {
    Quantified,
    UntilEndOfTurn,
    ConditionalPriorResult,
    Replacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalReplacementSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NextCastTriggerSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationCostHeadSurface {
    Leaf(leaf::LeafActivationCostHead),
    ManaGroup,
    Untap,
    Unattach,
    Signed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedSourcePrefixSurface {
    pub(crate) tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommaSplitSurface {
    pub(crate) head: String,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NamedReferenceSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceAliasEffectVerbSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedPriorObjectDiesSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealFirstDrawSurface {
    MandatoryEachTurn,
    MandatoryOwnTurns,
    OptionalEachTurn,
    OptionalOwnTurns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealFirstDrawFollowupSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsupportedLineHeadSurface {
    ModalChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HalfStartingLifePlusOneSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CumulativeUpkeepSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceLeavesBattlefieldSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StaticEffectContinuesUntilEndOfTurnSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThisPermanentSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhenOneOrMoreSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhenOneOrMoreThisWayFollowupSurface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedSourceEntersSurface {
    pub(crate) tail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AliasFaceSeparatorSurface;

pub(crate) fn parse_trailing_trigger_cap_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerCapSuffixShape<'_>> {
    const CAPS: &[(&[&str], TriggerCapSurface)] = &[
        (
            &[
                "this", "ability", "triggers", "only", "once", "each", "turn",
            ],
            TriggerCapSurface::Once,
        ),
        (
            &["do", "this", "only", "once", "each", "turn"],
            TriggerCapSurface::Once,
        ),
        (
            &[
                "this", "ability", "triggers", "only", "twice", "each", "turn",
            ],
            TriggerCapSurface::Twice,
        ),
        (
            &["do", "this", "only", "twice", "each", "turn"],
            TriggerCapSurface::Twice,
        ),
    ];
    for (phrase, cap) in CAPS {
        let Some(head) = primitives::strip_lexed_suffix_phrase(tokens, phrase) else {
            continue;
        };
        if head
            .last()
            .is_some_and(|token| token.kind == TokenKind::Quote)
        {
            if head
                .get(head.len().saturating_sub(2))
                .is_some_and(|token| token.kind == TokenKind::Period)
            {
                return Some(TriggerCapSuffixShape {
                    head_tokens: head,
                    cap: *cap,
                });
            }
            continue;
        }
        if !head
            .last()
            .is_some_and(|token| token.kind == TokenKind::Period)
        {
            continue;
        }
        return Some(TriggerCapSuffixShape {
            head_tokens: &head[..head.len().saturating_sub(1)],
            cap: *cap,
        });
    }
    None
}

pub(crate) fn parse_named_option_choice_header(
    tokens: &[OwnedLexToken],
) -> Option<NamedOptionChoiceHeader> {
    let words = TokenWordView::new(tokens).word_refs();
    let as_index = permission_shapes::find_words(&words, &["as"])?;
    let after_as = words.get(as_index + 1..)?;
    let enters_index = permission_shapes::find_words(after_as, &["enters"])?;
    let after_enters = after_as.get(enters_index + 1..)?;
    let choose_index = permission_shapes::find_words(after_enters, &["choose"])?;
    let after_choose = after_enters.get(choose_index + 1..)?;
    permission_shapes::find_words(after_choose, &["or"])?;
    Some(NamedOptionChoiceHeader)
}

pub(crate) fn parse_blocked_keyword_action_surface(tokens: &[OwnedLexToken]) -> Option<()> {
    let words = TokenWordView::new(tokens).word_refs();
    (permission_shapes::suffix_words(&words, &["cant", "be", "blocked"])
        && !permission_shapes::prefix_words(&words, &["this"])
        && !permission_shapes::prefix_words(&words, &["it"]))
    .then_some(())
}

pub(crate) fn parse_nonpermanent_statement_surface(
    tokens: &[OwnedLexToken],
) -> Option<NonpermanentStatementSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    if permission_shapes::prefix_words(&words, &["each"])
        || permission_shapes::prefix_words(&words, &["all"])
    {
        Some(NonpermanentStatementSurface::Quantified)
    } else if permission_shapes::find_words(&words, &["until", "end", "of", "turn"]).is_some() {
        Some(NonpermanentStatementSurface::UntilEndOfTurn)
    } else if matches!(
        super::semantic_lowering::parse_statement_effect_preference_tokens(tokens),
        Some(super::semantic_lowering::StatementEffectPreference::ConditionalPriorResult)
    ) {
        // A labeled nonpermanent-spell line such as "Spell mastery — If ...,
        // that creature enters ..." resolves against the preceding spell
        // instruction. Do not let its independently valid static parse detach
        // it from that prior result.
        Some(NonpermanentStatementSurface::ConditionalPriorResult)
    } else if permission_shapes::prefix_words(&words, &["if"])
        && permission_shapes::find_words(&words, &["instead"]).is_some()
    {
        Some(NonpermanentStatementSurface::Replacement)
    } else {
        None
    }
}

pub(crate) fn parse_conditional_replacement_surface(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalReplacementSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    (permission_shapes::prefix_words(&words, &["if"])
        && permission_shapes::find_words(&words, &["instead"]).is_some())
    .then_some(ConditionalReplacementSurface)
}

pub(crate) fn parse_next_cast_trigger_surface(
    tokens: &[OwnedLexToken],
) -> Option<NextCastTriggerSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    ((permission_shapes::prefix_words(&words, &["when"])
        || permission_shapes::prefix_words(&words, &["whenever"]))
        && permission_shapes::find_words(&words, &["next", "cast"]).is_some()
        && permission_shapes::find_words(&words, &["this", "turn"]).is_some())
    .then_some(NextCastTriggerSurface)
}

pub(crate) fn parse_activation_cost_head(
    tokens: &[OwnedLexToken],
) -> Option<ActivationCostHeadSurface> {
    match recognize_activation_cost_head(tokens) {
        ParseOutcome::Match(matched) => Some(matched.value),
        ParseOutcome::NoMatch | ParseOutcome::Error(_) => None,
    }
}

pub(crate) fn recognize_activation_cost_head(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<ActivationCostHeadSurface> {
    match leaf::recognize_activation_cost_head(tokens) {
        ParseOutcome::Match(matched) => {
            ParseOutcome::matched(ActivationCostHeadSurface::Leaf(matched.value), matched.span)
        }
        ParseOutcome::Error(diagnostic) => ParseOutcome::Error(diagnostic),
        ParseOutcome::NoMatch => {
            match primitives::parse_prefix(tokens, additional_activation_cost_head) {
                Some((head, _)) => {
                    ParseOutcome::matched(head, primitives::token_slice_span(tokens))
                }
                None => ParseOutcome::NoMatch,
            }
        }
    }
}

pub(crate) fn parse_source_alias_effect_verb_surface(
    alias: &str,
    remainder: &str,
) -> Option<SourceAliasEffectVerbSurface> {
    let alias_tokens = lex_line(alias.trim(), 0).ok()?;
    let mut alias_input = LexStream::new(&alias_tokens);
    source_alias_effect_verb.parse_next(&mut alias_input).ok()?;
    let ended: WResult<()> = eof.void().parse_next(&mut alias_input);
    ended.ok()?;

    let remainder_tokens = lex_line(remainder.trim(), 0).ok()?;
    let (_, next_word, _) =
        primitives::find_prefix(&remainder_tokens, || primitives::word_parser_text)?;
    (!matches!(
        next_word,
        "gets"
            | "get"
            | "has"
            | "have"
            | "is"
            | "are"
            | "enters"
            | "attacks"
            | "blocks"
            | "becomes"
            | "become"
            | "can't"
            | "cant"
            | "can"
            | "does"
            | "doesn't"
            | "doesnt"
    ))
    .then_some(SourceAliasEffectVerbSurface)
}

pub(crate) fn parse_named_source_prefix(
    text: &str,
    name: &str,
) -> Option<NamedSourcePrefixSurface> {
    let text_tokens = lex_line(text.trim(), 0).ok()?;
    let name_tokens = lex_line(name.trim(), 0).ok()?;
    let name_words = TokenWordView::new(&name_tokens).word_refs();
    if name_words.is_empty() {
        return None;
    }
    let text_words = TokenWordView::new(&text_tokens);
    if !permission_shapes::prefix_words(&text_words.word_refs(), &name_words) {
        return None;
    }
    let tail_token_index = text_words.token_index_after_words(name_words.len())?;
    let tail = render_token_slice(text_tokens.get(tail_token_index..)?)
        .trim_start()
        .to_string();
    (!tail.is_empty()).then_some(NamedSourcePrefixSurface { tail })
}

pub(crate) fn parse_first_comma(text: &str) -> Option<CommaSplitSurface> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    let (comma_index, _, _) = primitives::find_prefix(&tokens, || primitives::comma().void())?;
    let head = render_token_slice(tokens.get(..comma_index)?)
        .trim()
        .to_string();
    let body = render_token_slice(tokens.get(comma_index + 1..)?)
        .trim_start()
        .to_string();
    (!head.is_empty() && !body.is_empty()).then_some(CommaSplitSurface { head, body })
}

pub(crate) fn parse_named_reference(text: &str) -> Option<NamedReferenceSurface> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    primitives::find_prefix(&tokens, || primitives::kw("named")).map(|_| NamedReferenceSurface)
}

pub(crate) fn parse_alias_face_separator(alias: &str) -> Option<AliasFaceSeparatorSurface> {
    let mut input = alias;
    alias_face_separator.parse_next(&mut input).ok()?;
    Some(AliasFaceSeparatorSurface)
}

pub(crate) fn parse_delayed_prior_object_dies_surface(
    tokens: &[OwnedLexToken],
) -> Option<DelayedPriorObjectDiesSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    if !permission_shapes::prefix_words(&words, &["when", "that"])
        && !permission_shapes::prefix_words(&words, &["when", "it"])
    {
        return None;
    }
    permission_shapes::find_words(&words, &["dies", "this", "turn"])?;
    Some(DelayedPriorObjectDiesSurface)
}

pub(crate) fn parse_reveal_first_draw_surface(
    tokens: &[OwnedLexToken],
) -> Option<RevealFirstDrawSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    if permission_shapes::exact_words(
        &words,
        &[
            "reveal", "the", "first", "card", "you", "draw", "each", "turn",
        ],
    ) {
        Some(RevealFirstDrawSurface::MandatoryEachTurn)
    } else if permission_shapes::exact_words(
        &words,
        &[
            "reveal", "the", "first", "card", "you", "draw", "on", "each", "of", "your", "turns",
        ],
    ) {
        Some(RevealFirstDrawSurface::MandatoryOwnTurns)
    } else if permission_shapes::exact_words(
        &words,
        &[
            "you", "may", "reveal", "the", "first", "card", "you", "draw", "each", "turn", "as",
            "you", "draw", "it",
        ],
    ) {
        Some(RevealFirstDrawSurface::OptionalEachTurn)
    } else if permission_shapes::exact_words(
        &words,
        &[
            "you", "may", "reveal", "the", "first", "card", "you", "draw", "on", "each", "of",
            "your", "turns", "as", "you", "draw", "it",
        ],
    ) {
        Some(RevealFirstDrawSurface::OptionalOwnTurns)
    } else {
        None
    }
}

pub(crate) fn parse_reveal_first_draw_followup_surface(
    tokens: &[OwnedLexToken],
) -> Option<RevealFirstDrawFollowupSurface> {
    permission_shapes::prefix_tokens(tokens, &["whenever", "you", "reveal"])
        .then_some(RevealFirstDrawFollowupSurface)
}

pub(crate) fn parse_unsupported_line_head(
    tokens: &[OwnedLexToken],
) -> Option<UnsupportedLineHeadSurface> {
    permission_shapes::prefix_tokens(tokens, &["choose"])
        .then_some(UnsupportedLineHeadSurface::ModalChoice)
}

pub(crate) fn parse_half_starting_life_plus_one_surface(
    tokens: &[OwnedLexToken],
) -> Option<HalfStartingLifePlusOneSurface> {
    permission_shapes::contains_tokens(
        tokens,
        &[
            "if", "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half",
            "your", "starting", "life", "total", "plus", "one",
        ],
    )
    .then_some(HalfStartingLifePlusOneSurface)
}

pub(crate) fn parse_cumulative_upkeep_surface(
    tokens: &[OwnedLexToken],
) -> Option<CumulativeUpkeepSurface> {
    permission_shapes::prefix_tokens(tokens, &["cumulative", "upkeep"])
        .then_some(CumulativeUpkeepSurface)
}

pub(crate) fn parse_source_leaves_battlefield_surface(
    tokens: &[OwnedLexToken],
) -> Option<SourceLeavesBattlefieldSurface> {
    permission_shapes::contains_tokens(tokens, &["leaves", "the", "battlefield"])
        .then_some(SourceLeavesBattlefieldSurface)
}

pub(crate) fn parse_static_effect_continues_until_end_of_turn_surface(
    tokens: &[OwnedLexToken],
) -> Option<StaticEffectContinuesUntilEndOfTurnSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    if words.len() != 13
        || words[0] != "if"
        || words[1] != "this"
        || !matches!(
            words[2],
            "artifact" | "creature" | "enchantment" | "land" | "permanent"
        )
        || !permission_shapes::exact_words(
            &words[3..],
            &[
                "leaves",
                "the",
                "battlefield",
                "this",
                "effect",
                "continues",
                "until",
                "end",
                "of",
                "turn",
            ],
        )
    {
        return None;
    }
    Some(StaticEffectContinuesUntilEndOfTurnSurface)
}

pub(crate) fn parse_this_permanent_surface(
    tokens: &[OwnedLexToken],
) -> Option<ThisPermanentSurface> {
    permission_shapes::contains_tokens(tokens, &["this", "permanent"])
        .then_some(ThisPermanentSurface)
}

pub(crate) fn parse_when_one_or_more_surface(
    tokens: &[OwnedLexToken],
) -> Option<WhenOneOrMoreSurface> {
    (permission_shapes::prefix_tokens(tokens, &["when", "one", "or", "more"])
        || permission_shapes::prefix_tokens(tokens, &["whenever", "one", "or", "more"]))
    .then_some(WhenOneOrMoreSurface)
}

pub(crate) fn parse_when_one_or_more_this_way_followup_surface(
    tokens: &[OwnedLexToken],
) -> Option<WhenOneOrMoreThisWayFollowupSurface> {
    primitives::parse_prefix(tokens, when_one_or_more_followup_head)?;
    let before_comma = primitives::find_prefix(tokens, || primitives::comma().void())
        .and_then(|(comma, _, _)| tokens.get(..comma))
        .unwrap_or(tokens);
    permission_shapes::contains_tokens(before_comma, &["this", "way"])
        .then_some(WhenOneOrMoreThisWayFollowupSurface)
}

pub(crate) fn parse_named_source_enters_surface(text: &str) -> Option<NamedSourceEntersSurface> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    let (_, _, tail_tokens) = primitives::find_prefix(&tokens, || primitives::kw("enters").void())?;
    if tail_tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        return None;
    }
    let tail = render_token_slice(tail_tokens).trim_start().to_string();
    (!tail.is_empty()).then_some(NamedSourceEntersSurface { tail })
}

#[cfg(test)]
mod nonpermanent_prior_result_tests {
    use super::*;

    #[test]
    fn explicit_prior_result_conditionals_are_nonpermanent_statements() {
        for text in [
            "If X is 6 or more, those permanents are 4/4 creatures in addition to their other types.",
            "If there are two or more instant and/or sorcery cards in your graveyard, that creature enters with two additional +1/+1 counters on it.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_nonpermanent_statement_surface(&tokens),
                Some(NonpermanentStatementSurface::ConditionalPriorResult),
                "{text}"
            );
        }

        for text in [
            "If you control a Plains, creatures you control get +1/+1.",
            "If this creature entered this turn, it has haste.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_nonpermanent_statement_surface(&tokens),
                None,
                "{text}"
            );
        }
    }
}

#[cfg(test)]
#[path = "document_shapes/tests.rs"]
mod tests;
