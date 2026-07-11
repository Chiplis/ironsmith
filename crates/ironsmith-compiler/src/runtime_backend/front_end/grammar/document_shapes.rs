use winnow::combinator::{alt, eof, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal, take_till};

use super::{leaf, permission_shapes, primitives};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerCapSurface {
    Once,
    Twice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NamedOptionChoiceHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonpermanentStatementSurface {
    Quantified,
    UntilEndOfTurn,
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

pub(crate) fn parse_trigger_cap(words: &[&str]) -> Option<TriggerCapSurface> {
    if permission_shapes::exact_words(
        words,
        &[
            "this", "ability", "triggers", "only", "once", "each", "turn",
        ],
    ) || permission_shapes::exact_words(words, &["do", "this", "only", "once", "each", "turn"])
    {
        Some(TriggerCapSurface::Once)
    } else if permission_shapes::exact_words(
        words,
        &[
            "this", "ability", "triggers", "only", "twice", "each", "turn",
        ],
    ) || permission_shapes::exact_words(
        words,
        &["do", "this", "only", "twice", "each", "turn"],
    ) {
        Some(TriggerCapSurface::Twice)
    } else {
        None
    }
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
    if let Some(head) = leaf::parse_leaf_activation_cost_head_tokens(tokens) {
        return Some(ActivationCostHeadSurface::Leaf(head));
    }
    primitives::parse_prefix(tokens, additional_activation_cost_head).map(|(head, _)| head)
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

fn when_one_or_more_followup_head(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            alt((primitives::kw("when"), primitives::kw("whenever"))),
            primitives::phrase(&["one", "or", "more"]),
        )
            .void(),
        (
            alt((primitives::kw("when"), primitives::kw("whenever"))),
            primitives::kw("you"),
            alt((
                primitives::kw("discard"),
                primitives::kw("exile"),
                primitives::kw("mill"),
                primitives::kw("sacrifice"),
            )),
            primitives::phrase(&["one", "or", "more"]),
        )
            .void(),
    ))
    .parse_next(input)
}

fn additional_activation_cost_head(
    input: &mut LexStream<'_>,
) -> WResult<ActivationCostHeadSurface> {
    let token: &OwnedLexToken = any.parse_next(input)?;
    if token.kind == TokenKind::ManaGroup {
        return Ok(ActivationCostHeadSurface::ManaGroup);
    }
    if matches!(token.kind, TokenKind::Plus | TokenKind::Dash) {
        return Ok(ActivationCostHeadSurface::Signed);
    }
    match token.parser_text() {
        "untap" => Ok(ActivationCostHeadSurface::Untap),
        "unattach" => Ok(ActivationCostHeadSurface::Unattach),
        signed if token.kind == TokenKind::Word && parse_signed_word(signed) => {
            Ok(ActivationCostHeadSurface::Signed)
        }
        _ => Err(primitives::backtrack_err(
            "activation-cost head",
            "signed loyalty or activation-cost verb",
        )),
    }
}

fn alias_face_separator(input: &mut &str) -> WResult<()> {
    let _: &str = take_till(0.., |character: char| character == '/').parse_next(input)?;
    literal('/').void().parse_next(input)
}

fn parse_signed_word(raw: &str) -> bool {
    let mut input = raw;
    let parsed: WResult<()> = (
        alt((literal('+'), literal('-'))),
        repeat::<_, _, (), _, _>(1.., any.void()),
        eof,
    )
        .void()
        .parse_next(&mut input);
    parsed.is_ok()
}

fn source_alias_effect_verb(input: &mut LexStream<'_>) -> WResult<()> {
    let word = primitives::word_parser_text.parse_next(input)?;
    matches!(
        word,
        "add"
            | "attach"
            | "become"
            | "convert"
            | "counter"
            | "create"
            | "deal"
            | "destroy"
            | "detain"
            | "discard"
            | "draw"
            | "exchange"
            | "exile"
            | "get"
            | "goad"
            | "incubate"
            | "investigate"
            | "look"
            | "mill"
            | "move"
            | "pay"
            | "proliferate"
            | "regenerate"
            | "remove"
            | "return"
            | "sacrifice"
            | "scry"
            | "shuffle"
            | "skip"
            | "surveil"
            | "suspect"
            | "tap"
            | "transform"
            | "untap"
    )
    .then_some(())
    .ok_or_else(|| primitives::backtrack_err("source alias", "effect verb"))
}

#[cfg(test)]
#[path = "document_shapes/tests.rs"]
mod tests;
