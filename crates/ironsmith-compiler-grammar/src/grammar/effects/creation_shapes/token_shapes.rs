use std::ops::Range;

use winnow::ascii::{dec_int, digit1};
use winnow::combinator::{alt, eof, opt, peek, repeat_till, separated_pair};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::{EventValueSpec, Value};
use crate::target::PlayerFilter;

use super::super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, token_word_refs, trim_lexed_commas,
};
use super::super::super::{leaf, primitives};
use super::{CreationPhrase, CreationTokens, CreationWordClass, CreationWords};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedCombatTokenAction {
    Exile,
    Sacrifice,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrailingCreateDelay {
    pub start_word: usize,
    pub player: PlayerFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopySourceTailSpec {
    pub source_tokens: Vec<OwnedLexToken>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineCombatSpec {
    pub source_tokens: Vec<OwnedLexToken>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub attacks_that_player_or_planeswalker: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopySourceClauseSpec {
    pub source_tokens: Vec<OwnedLexToken>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub attacks_that_player_or_planeswalker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqualToCountClause<'a> {
    pub value_tokens: &'a [OwnedLexToken],
    pub cut_token: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreateCountHead {
    Default,
    EventAmount,
    EqualToDynamic,
    Dynamic(Value),
    X,
    Fixed(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateHeadSpec<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub count: CreateCountHead,
    pub name_words: Vec<&'a str>,
    pub name_tokens: &'a [OwnedLexToken],
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedTokenClauseShape {
    pub clause: Range<usize>,
    pub name: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttachmentClause<'a> {
    pub prefix_tokens: &'a [OwnedLexToken],
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForEachClause<'a> {
    pub prefix_tokens: &'a [OwnedLexToken],
    pub filter_tokens: &'a [OwnedLexToken],
    pub start_word: usize,
}

#[path = "token_shapes/pt.rs"]
mod pt;
pub use pt::*;

pub fn parse_delayed_combat_token_action_words(words: &[&str]) -> Option<DelayedCombatTokenAction> {
    let surface = CreationWords::new(words);
    if words.len() != 7
        || !surface.class_at(1, CreationWordClass::InlineReference)
        || !surface.class_at(2, CreationWordClass::Token)
        || !CreationWords::new(words.get(3..)?).exact(CreationPhrase::AtEndOfCombat)
    {
        return None;
    }
    if words.first().copied() == Some("exile") {
        Some(DelayedCombatTokenAction::Exile)
    } else if words.first().copied() == Some("sacrifice") {
        Some(DelayedCombatTokenAction::Sacrifice)
    } else {
        None
    }
}

pub fn parse_trailing_create_delay_words(words: &[&str]) -> Option<TrailingCreateDelay> {
    let surface = CreationWords::new(words);
    let start_word = surface.suffix_location(CreationPhrase::TrailingNextEndStep)?;
    if CreationWords::new(&words[..start_word]).has(CreationWordClass::When) {
        return None;
    }
    let player = if words.get(start_word + 5).copied() == Some("your") {
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };
    Some(TrailingCreateDelay { start_word, player })
}

pub fn parse_copy_source_tail_tokens(tokens: &[OwnedLexToken]) -> CopySourceTailSpec {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let mut cursor = 0usize;
    let split_word = loop {
        let Some(relative) = CreationWords::new(&words[cursor..]).location(CreationWordClass::And)
        else {
            break None;
        };
        let candidate = cursor + relative;
        let tail = words.get(candidate + 1..).unwrap_or_default();
        let tail_surface = CreationWords::new(tail);
        if tail_surface.first_is(CreationWordClass::InlineReference)
            && (tail_surface.has(CreationWordClass::Tapped)
                || tail_surface.has(CreationWordClass::Attacking))
        {
            break Some(candidate);
        }
        cursor = candidate + 1;
    };

    let Some(split_word) = split_word else {
        return CopySourceTailSpec {
            source_tokens: tokens.to_vec(),
            enters_tapped: false,
            enters_attacking: false,
        };
    };
    let split_token = token_surface.boundary(split_word).unwrap_or(tokens.len());
    let modifier_token = token_surface
        .boundary(split_word + 1)
        .unwrap_or(tokens.len());
    let modifier_words = CreationTokens::new(trim_lexed_commas(&tokens[modifier_token..])).words();
    let modifier_surface = CreationWords::new(&modifier_words);
    CopySourceTailSpec {
        source_tokens: trim_lexed_commas(&tokens[..split_token]).to_vec(),
        enters_tapped: modifier_surface.has(CreationWordClass::Tapped),
        enters_attacking: modifier_surface.has(CreationWordClass::Attacking),
    }
}

pub fn parse_inline_combat_tokens(tokens: &[OwnedLexToken]) -> InlineCombatSpec {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    let Some(start_word) = surface.phrase_location(CreationPhrase::InlineModifierStart) else {
        return InlineCombatSpec {
            source_tokens: tokens.to_vec(),
            enters_tapped: false,
            enters_attacking: false,
            attacks_that_player_or_planeswalker: false,
        };
    };
    let modifier_words = &words[start_word..];
    let modifier_surface = CreationWords::new(modifier_words);
    let enters_tapped = modifier_surface.has(CreationWordClass::Tapped);
    let enters_attacking = modifier_surface.has(CreationWordClass::Attacking);
    if !enters_tapped && !enters_attacking {
        return InlineCombatSpec {
            source_tokens: tokens.to_vec(),
            enters_tapped: false,
            enters_attacking: false,
            attacks_that_player_or_planeswalker: false,
        };
    }
    let start_token = token_surface.boundary(start_word).unwrap_or(tokens.len());
    InlineCombatSpec {
        source_tokens: trim_lexed_commas(&tokens[..start_token]).to_vec(),
        enters_tapped,
        enters_attacking,
        attacks_that_player_or_planeswalker: modifier_surface
            .has_phrase(CreationPhrase::AttackTarget),
    }
}

pub fn parse_copy_source_clause_tokens(tokens: &[OwnedLexToken]) -> Option<CopySourceClauseSpec> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let of_word = CreationWords::new(&words).location(CreationWordClass::Of)?;
    let source_start = token_surface.boundary(of_word + 1)?;
    let source_tail = tokens.get(source_start..)?;
    let mut source_end = source_tail.len();
    for (idx, token) in source_tail.iter().enumerate() {
        let is_except =
            CreationTokens::new(std::slice::from_ref(token)).token_is(0, CreationWordClass::Except);
        let comma_before_except = token.is_comma()
            && source_tail.get(idx + 1).is_some_and(|next| {
                CreationTokens::new(std::slice::from_ref(next))
                    .token_is(0, CreationWordClass::Except)
            });
        if is_except || comma_before_except {
            source_end = idx;
            break;
        }
    }
    for idx in 1..source_end {
        let suffix_words = CreationTokens::new(&source_tail[idx..source_end]).words();
        let suffix = CreationWords::new(&suffix_words);
        let after_and_words = CreationTokens::new(&source_tail[idx + 1..source_end]).words();
        if suffix.starts(CreationPhrase::InlineRulesTail)
            || (suffix.first_is(CreationWordClass::And)
                && CreationWords::new(&after_and_words).starts(CreationPhrase::InlineRulesTail))
        {
            source_end = idx;
            break;
        }
    }
    let source = source_tail.get(..source_end)?;
    let tail = parse_copy_source_tail_tokens(source);
    let inline = parse_inline_combat_tokens(&tail.source_tokens);
    Some(CopySourceClauseSpec {
        source_tokens: inline.source_tokens,
        enters_tapped: tail.enters_tapped || inline.enters_tapped,
        enters_attacking: tail.enters_attacking || inline.enters_attacking,
        attacks_that_player_or_planeswalker: inline.attacks_that_player_or_planeswalker,
    })
}

pub fn parse_equal_to_count_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EqualToCountClause<'_>> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let equal_word = CreationWords::new(&words).phrase_location(CreationPhrase::EqualTo)?;
    let cut_token = token_surface.boundary(equal_word)?;
    let value_token = token_surface.boundary(equal_word + 2)?;
    let value_tokens = trim_lexed_commas(tokens.get(value_token..)?);
    (!value_tokens.is_empty()).then_some(EqualToCountClause {
        value_tokens,
        cut_token,
    })
}

pub fn creation_body_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if CreationTokens::new(tokens).token_is(0, CreationWordClass::Create) {
        &tokens[1..]
    } else {
        tokens
    }
}

pub fn parse_create_head_tokens(tokens: &[OwnedLexToken]) -> Option<CreateHeadSpec<'_>> {
    let body_tokens = creation_body_tokens(tokens);
    let token_surface = CreationTokens::new(body_tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    let (count, mut consumed_words) = if words.get(..2) == Some(&["that", "many"][..]) {
        (CreateCountHead::EventAmount, 2)
    } else if surface.starts(CreationPhrase::NumberOf) {
        (CreateCountHead::EqualToDynamic, 3)
    } else if words.first().copied() == Some("x") {
        (CreateCountHead::X, 1)
    } else if words.first().copied() == Some("twice")
        && let Some((value, used)) =
            super::super::super::shared_util::value_expr::parse_value_expr_words(&words)
                .filter(|(value, _)| !matches!(value, Value::Fixed(_)))
    {
        (CreateCountHead::Dynamic(value), used)
    } else if let Some(prefix) = leaf::parse_leaf_number_prefix_words(&words) {
        let (count, used) = prefix.into_fixed()?;
        (CreateCountHead::Fixed(count), used)
    } else if CreationWords::new(&words).first_is(CreationWordClass::Article) {
        // An article begins the token blueprint, never a dynamic count. Keep
        // the general value-expression parser out of this overwhelmingly
        // common default-count path.
        (CreateCountHead::Default, 0)
    } else if let Some((value, used)) =
        super::super::super::shared_util::value_expr::parse_value_expr_words(&words)
            .filter(|(value, _)| !matches!(value, Value::Fixed(_)))
    {
        (CreateCountHead::Dynamic(value), used)
    } else {
        (CreateCountHead::Default, 0)
    };
    if CreationWords::new(words.get(consumed_words..)?).first_is(CreationWordClass::Article) {
        consumed_words += 1;
    }
    let remaining = words.get(consumed_words..)?;
    let marker_offset = CreationWords::new(remaining).location(CreationWordClass::Token)?;
    let marker_word = consumed_words + marker_offset;
    let name_start = token_surface.boundary(consumed_words)?;
    let name_end = token_surface.boundary(marker_word)?;
    let tail_start = token_surface.boundary(marker_word + 1)?;
    let name_tokens = body_tokens.get(name_start..name_end)?;
    let surface_name_words = token_word_refs(name_tokens);
    Some(CreateHeadSpec {
        body_tokens,
        count,
        name_words: crate::util::non_article_word_refs(&surface_name_words),
        name_tokens,
        tail_tokens: body_tokens.get(tail_start..)?,
    })
}

/// Split a three-or-more token creation operand list whose action is authored
/// only once: `a Clue token, a Food token, and a Junk token`.
///
/// Requiring at least two top-level commas and a final `and` keeps commas in
/// named token appositives and token rules out of this shape. Each member must
/// independently be a complete simple token head when supplied the carried
/// `create` action.
pub fn parse_serial_create_token_operand_list_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<&[OwnedLexToken]>> {
    let mut comma_indices = Vec::new();
    let mut inside_quote = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quote = !inside_quote;
            continue;
        }
        if !inside_quote && token.kind == TokenKind::Comma {
            comma_indices.push(index);
        }
    }
    if comma_indices.len() < 2 {
        return None;
    }

    let mut operands = Vec::with_capacity(comma_indices.len() + 1);
    let mut start = 0usize;
    for comma in &comma_indices {
        let operand = trim_lexed_commas(tokens.get(start..*comma)?);
        if operand.is_empty() {
            return None;
        }
        operands.push(operand);
        start = comma + 1;
    }
    let tail = trim_lexed_commas(tokens.get(start..)?);
    let tail = tail
        .first()
        .is_some_and(|token| token.is_word("and"))
        .then(|| trim_lexed_commas(&tail[1..]))?;
    if tail.is_empty() {
        return None;
    }
    operands.push(tail);

    for operand in &operands {
        let mut clause = Vec::with_capacity(operand.len() + 1);
        clause.push(OwnedLexToken::synthetic_word("create"));
        clause.extend_from_slice(operand);
        let head = parse_create_head_tokens(&clause)?;
        if head.name_tokens.is_empty()
            || head
                .tail_tokens
                .iter()
                .any(|token| !matches!(token.kind, TokenKind::Period))
        {
            return None;
        }
    }

    Some(operands)
}

fn apostrophe<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::token_kind(TokenKind::Apostrophe)
        .void()
        .parse_next(input)
}

fn quoted_token_group<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            primitives::quote(),
            repeat_till::<_, _, (), _, _, _, _>(
                0..,
                any.void(),
                alt((peek(primitives::quote()).void(), eof.void())),
            )
            .void(),
            opt(primitives::quote()).void(),
        )
            .void(),
        (
            apostrophe,
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(apostrophe)).void(),
            apostrophe,
        )
            .void(),
    ))
    .parse_next(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnquotedForEachMarker {
    start_token: usize,
    filter_start_token: usize,
}

fn double_quoted_token_group<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::quote(),
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            alt((peek(primitives::quote()).void(), eof.void())),
        )
        .void(),
        opt(primitives::quote()).void(),
    )
        .void()
        .parse_next(input)
}

fn parse_unquoted_for_each_marker(input: &mut LexStream<'_>) -> WResult<UnquotedForEachMarker> {
    let initial_len = input.len();
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        alt((double_quoted_token_group, any.void())),
        peek(primitives::phrase(&["for", "each"])),
    )
    .void()
    .parse_next(input)?;
    let start_token = initial_len.saturating_sub(input.len());
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    Ok(UnquotedForEachMarker {
        start_token,
        filter_start_token: initial_len.saturating_sub(input.len()),
    })
}

fn non_named_token<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| !token.is_word("named"))
        .void()
        .parse_next(input)
}

fn token_name_boundary<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("with").void(),
        primitives::kw("that").void(),
        primitives::kw("which").void(),
        primitives::kw("thats").void(),
        primitives::phrase(&["for", "each"]),
        eof.void(),
    ))
    .parse_next(input)
}

fn parse_named_token_clause(input: &mut LexStream<'_>) -> WResult<NamedTokenClauseShape> {
    let initial_len = input.len();
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        alt((quoted_token_group, non_named_token)),
        peek(primitives::kw("named")),
    )
    .void()
    .parse_next(input)?;
    let marker_start = initial_len.saturating_sub(input.len());
    primitives::kw("named").parse_next(input)?;
    let name_start = initial_len.saturating_sub(input.len());
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(token_name_boundary))
        .void()
        .parse_next(input)?;
    let name_end = initial_len.saturating_sub(input.len());
    Ok(NamedTokenClauseShape {
        clause: marker_start..name_end,
        name: name_start..name_end,
    })
}

pub fn parse_named_token_clause_tokens(tokens: &[OwnedLexToken]) -> Option<NamedTokenClauseShape> {
    let mut input = LexStream::new(tokens);
    parse_named_token_clause.parse_next(&mut input).ok()
}

pub fn parse_attachment_clause_tokens(tokens: &[OwnedLexToken]) -> Option<AttachmentClause<'_>> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    let attached = surface.phrase_location(CreationPhrase::AttachedTo)?;
    if surface
        .phrase_location(CreationPhrase::ForEach)
        .is_some_and(|for_each| for_each < attached)
    {
        return None;
    }
    let prefix_end = token_surface.boundary(attached)?;
    let target_start = token_surface.boundary(attached + 2)?;
    let target_tokens = trim_lexed_commas(tokens.get(target_start..)?);
    (!target_tokens.is_empty()).then_some(AttachmentClause {
        prefix_tokens: trim_lexed_commas(&tokens[..prefix_end]),
        target_tokens,
    })
}

pub fn parse_for_each_clause_tokens(tokens: &[OwnedLexToken]) -> Option<ForEachClause<'_>> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    let marker = primitives::parse_prefix(tokens, parse_unquoted_for_each_marker)?.0;
    let word_view = TokenWordView::new(tokens);
    let start_word = word_view
        .token_start_indices()
        .binary_search(&marker.start_token)
        .ok()?;
    let prefix_words = &words[..start_word];
    let prefix = CreationWords::new(prefix_words);
    if prefix.has_phrase(CreationPhrase::TokenRulesText)
        || (prefix.has(CreationWordClass::Token) && prefix.has(CreationWordClass::GrantVerb))
    {
        return None;
    }
    if let Some(with_word) = surface.location(CreationWordClass::With)
        && with_word < start_word
    {
        let between = CreationWords::new(&words[with_word + 1..start_word]);
        if between.has(CreationWordClass::RulesTextStart) || between.has(CreationWordClass::Counter)
        {
            return None;
        }
    }
    let filter_tokens = trim_lexed_commas(tokens.get(marker.filter_start_token..)?);
    Some(ForEachClause {
        prefix_tokens: trim_lexed_commas(&tokens[..marker.start_token]),
        filter_tokens,
        start_word,
    })
}

pub fn parse_for_each_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    if !CreationWords::new(&words).starts(CreationPhrase::ForEach) {
        return None;
    }
    let start = token_surface.boundary(2)?;
    Some(trim_lexed_commas(tokens.get(start..)?))
}

pub fn parse_where_clause_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let where_word = CreationWords::new(&words).location(CreationWordClass::Where)?;
    let start = token_surface.boundary(where_word)?;
    Some(trim_lexed_commas(tokens.get(start..)?))
}

pub fn parse_time_only_words(words: &[&str]) -> bool {
    let surface = CreationWords::new(words);
    words.is_empty() || (words.len() == 1 && surface.first_is(CreationWordClass::Time))
}

pub fn create_count_head_value(head: &CreateCountHead) -> Value {
    match head {
        CreateCountHead::Default => Value::Fixed(1),
        CreateCountHead::EventAmount => Value::EventValue(EventValueSpec::Amount),
        CreateCountHead::EqualToDynamic => Value::Fixed(1),
        CreateCountHead::Dynamic(value) => value.clone(),
        CreateCountHead::X => Value::X,
        CreateCountHead::Fixed(count) => Value::Fixed(*count as i32),
    }
}

#[cfg(test)]
#[path = "token_shapes_tests.rs"]
mod tests;
