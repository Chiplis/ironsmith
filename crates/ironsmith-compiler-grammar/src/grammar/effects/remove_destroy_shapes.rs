use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::{conditions, leaf, permission_shapes, primitives, values};
use super::unless_clause::{self, UnlessPaysShape};
use crate::effect::Value;
use crate::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveShapeError {
    MissingAmount,
    MissingCounterKeyword,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoveCounterDestination<'a> {
    EachOfAnyNumber {
        filter_tokens: &'a [OwnedLexToken],
    },
    All {
        filter_tokens: &'a [OwnedLexToken],
    },
    Among {
        filter_tokens: &'a [OwnedLexToken],
    },
    ForEach {
        target_tokens: &'a [OwnedLexToken],
        count_filter_tokens: &'a [OwnedLexToken],
        fallback_target_tokens: &'a [OwnedLexToken],
    },
    Single {
        target_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoveClauseShape<'a> {
    AllOfThem,
    FromCombat {
        target_tokens: &'a [OwnedLexToken],
    },
    AllCounters {
        counter_descriptor: &'a [OwnedLexToken],
        target_tokens: &'a [OwnedLexToken],
        source_like_target: bool,
        leave_one: bool,
    },
    Counters {
        amount: Value,
        up_to: bool,
        counter_descriptor: &'a [OwnedLexToken],
        destination: RemoveCounterDestination<'a>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedDestroyTimingShape {
    EndOfCombat,
    NextEndStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedDestroyRelation {
    Matching,
    ExceptMatching,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestroyAllShape<'a> {
    DealtDamageThisTurn {
        filter_tokens: &'a [OwnedLexToken],
    },
    DealtDamageToPlayerThisTurn {
        filter_tokens: &'a [OwnedLexToken],
        player_tokens: &'a [OwnedLexToken],
    },
    AttachedTo {
        filter_tokens: &'a [OwnedLexToken],
        target_tokens: &'a [OwnedLexToken],
    },
    ExceptFor {
        filter_tokens: &'a [OwnedLexToken],
        exception_tokens: &'a [OwnedLexToken],
    },
    ChosenColor {
        filter_tokens: &'a [OwnedLexToken],
    },
    ChosenThisWay {
        filter_tokens: &'a [OwnedLexToken],
        relation: TaggedDestroyRelation,
    },
    Plain {
        filter_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestroyCombatHistoryShape<'a> {
    DealtDamageThisTurn {
        target_tokens: &'a [OwnedLexToken],
    },
    DealtDamageToPlayerThisTurn {
        target_tokens: &'a [OwnedLexToken],
        player_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestroyTargetAndAttachedShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub attachment_filter_tokens: &'a [OwnedLexToken],
    pub demonstrative_antecedent: Option<ironsmith_core::DemonstrativeAntecedentSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestroyClauseKind<'a> {
    Empty,
    UnsupportedDelayedTiming,
    CombatHistory(DestroyCombatHistoryShape<'a>),
    UnsupportedCombatHistory,
    All(DestroyAllShape<'a>),
    UnlessTargetSetPredicate {
        target_tokens: &'a [OwnedLexToken],
        predicate: conditions::TargetSetPredicateAst,
    },
    UnlessPays {
        target_tokens: &'a [OwnedLexToken],
        payment: UnlessPaysShape<'a>,
    },
    UnsupportedUnless,
    TrailingAttackOrBlockRestriction,
    Conditional {
        target_tokens: &'a [OwnedLexToken],
        predicate_tokens: &'a [OwnedLexToken],
    },
    UnsupportedConditional,
    TargetAndAttached(DestroyTargetAndAttachedShape<'a>),
    InlineNoRegeneration {
        target_tokens: &'a [OwnedLexToken],
    },
    MultiTarget,
    Blocked {
        target_tokens: Vec<OwnedLexToken>,
    },
    Plain {
        target_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestroyClauseShape<'a> {
    pub timing: Option<DelayedDestroyTimingShape>,
    pub kind: DestroyClauseKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyCounterConstraintKind {
    With,
    Without,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyCounterConstraintShape<'a> {
    pub base_tokens: &'a [OwnedLexToken],
    pub constraint_tokens: &'a [OwnedLexToken],
    pub kind: DestroyCounterConstraintKind,
}

fn counter_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("counter").void(),
        primitives::kw("counters").void(),
    ))
    .parse_next(input)
}

fn all_or_each_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("all").void(), primitives::kw("each").void())).parse_next(input)
}

fn exact_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    permission_shapes::exact_tokens(tokens, expected)
}

fn trim_shape_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

fn source_like_remove_target(tokens: &[OwnedLexToken]) -> bool {
    [
        &["it"][..],
        &["this"],
        &["this", "creature"],
        &["this", "artifact"],
        &["this", "enchantment"],
        &["this", "permanent"],
        &["this", "card"],
    ]
    .iter()
    .any(|expected| exact_tokens(tokens, expected))
}

pub fn parse_remove_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<RemoveClauseShape<'_>, RemoveShapeError> {
    let tokens = trim_shape_edges(tokens);
    if exact_tokens(tokens, &["all", "of", "them"]) || exact_tokens(tokens, &["those", "counters"]) {
        return Ok(RemoveClauseShape::AllOfThem);
    }

    if let Some((target_tokens, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::phrase(&["from", "combat"])
    }) {
        return Ok(RemoveClauseShape::FromCombat {
            target_tokens: trim_lexed_commas(target_tokens),
        });
    }

    // The article in "a number of ... counters equal to ..." introduces a
    // dynamic amount; it is not the fixed amount one. Claim the complete
    // equality surface before the generic value-prefix parser can consume
    // only `a` and leave the amount expression inside the target phrase.
    if let Some(((), after_number_of)) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["a", "number", "of"]),
            primitives::phrase(&["the", "number", "of"]),
        ))
        .void(),
    ) && let Some((counter_idx, (), after_counter)) =
        primitives::find_prefix(after_number_of, || counter_word)
        && let Some(((), after_equal_to)) =
            primitives::parse_prefix(after_counter, primitives::phrase(&["equal", "to"]).void())
        && let Some((from_idx, (), target_tokens)) =
            primitives::find_prefix(after_equal_to, || primitives::kw("from").void())
    {
        let value_tokens = trim_lexed_commas(&after_equal_to[..from_idx]);
        let target_tokens = trim_lexed_commas(target_tokens);
        if !value_tokens.is_empty()
            && !target_tokens.is_empty()
            && let Some((amount, used)) = values::parse_value_prefix_lexed(value_tokens)
            && used == value_tokens.len()
        {
            return Ok(RemoveClauseShape::Counters {
                amount: amount.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                up_to: false,
                counter_descriptor: trim_lexed_commas(&after_number_of[..counter_idx]),
                destination: RemoveCounterDestination::Single { target_tokens },
            });
        }
    }

    if let Some(((), after_all)) = primitives::parse_prefix(tokens, primitives::kw("all").void())
        && let (leave_one, after_quantity) = if let Some(((), rest)) =
            primitives::parse_prefix(after_all, primitives::phrase(&["but", "one"]).void())
        {
            (true, rest)
        } else {
            (false, after_all)
        }
        && let Some((counter_idx, (), after_counter)) =
            primitives::find_prefix(after_quantity, || counter_word)
    {
        let counter_descriptor = trim_lexed_commas(&after_quantity[..counter_idx]);
        let target_tokens = if let Some(((), rest)) =
            primitives::parse_prefix(after_counter, primitives::kw("from").void())
        {
            trim_lexed_commas(rest)
        } else {
            trim_lexed_commas(after_counter)
        };
        return Ok(RemoveClauseShape::AllCounters {
            counter_descriptor,
            target_tokens,
            source_like_target: source_like_remove_target(target_tokens),
            leave_one,
        });
    }

    let (up_to, value_tokens) = if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["up", "to"]))
    {
        (true, rest)
    } else {
        (false, tokens)
    };
    let (amount, amount_used) =
        values::parse_value_prefix_lexed(value_tokens).ok_or(RemoveShapeError::MissingAmount)?;
    let after_amount = value_tokens
        .get(amount_used..)
        .ok_or(RemoveShapeError::MissingCounterKeyword)?;
    let (counter_idx, (), after_counter) = primitives::find_prefix(after_amount, || counter_word)
        .ok_or(RemoveShapeError::MissingCounterKeyword)?;
    let counter_descriptor = trim_lexed_commas(&after_amount[..counter_idx]);
    let target_tokens = if let Some(((), rest)) =
        primitives::parse_prefix(after_counter, primitives::kw("from").void())
    {
        trim_lexed_commas(rest)
    } else {
        trim_lexed_commas(after_counter)
    };

    let destination = if let Some(((), after_among)) =
        primitives::parse_prefix(target_tokens, primitives::kw("among").void())
    {
        let filter_tokens = if let Some(((), rest)) =
            primitives::parse_prefix(after_among, primitives::kw("all").void())
        {
            rest
        } else {
            after_among
        };
        RemoveCounterDestination::Among {
            filter_tokens: trim_lexed_commas(filter_tokens),
        }
    } else if let Some(((), filter_tokens)) = primitives::parse_prefix(
        target_tokens,
        primitives::phrase(&["each", "of", "any", "number", "of"]).void(),
    ) {
        RemoveCounterDestination::EachOfAnyNumber {
            filter_tokens: trim_lexed_commas(filter_tokens),
        }
    } else if let Some(((), filter_tokens)) =
        primitives::parse_prefix(target_tokens, all_or_each_word)
    {
        RemoveCounterDestination::All {
            filter_tokens: trim_lexed_commas(filter_tokens),
        }
    } else if let Some((for_each_idx, (), count_filter_tokens)) =
        primitives::find_prefix(target_tokens, || primitives::phrase(&["for", "each"]))
    {
        let base_target_tokens = trim_lexed_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_lexed_commas(count_filter_tokens);
        if base_target_tokens.is_empty() || count_filter_tokens.is_empty() {
            RemoveCounterDestination::Single { target_tokens }
        } else {
            RemoveCounterDestination::ForEach {
                target_tokens: base_target_tokens,
                count_filter_tokens,
                fallback_target_tokens: target_tokens,
            }
        }
    } else {
        RemoveCounterDestination::Single { target_tokens }
    };

    Ok(RemoveClauseShape::Counters {
        amount,
        up_to,
        counter_descriptor,
        destination,
    })
}

fn delayed_destroy_timing<'a>(input: &mut LexStream<'a>) -> WResult<DelayedDestroyTimingShape> {
    alt((
        alt((
            primitives::phrase(&["at", "end", "of", "combat"]),
            primitives::phrase(&["at", "the", "end", "of", "combat"]),
        ))
        .value(DelayedDestroyTimingShape::EndOfCombat),
        alt((
            primitives::phrase(&["at", "beginning", "of", "next", "end", "step"]),
            primitives::phrase(&["at", "beginning", "of", "the", "next", "end", "step"]),
            primitives::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
        ))
        .value(DelayedDestroyTimingShape::NextEndStep),
    ))
    .parse_next(input)
}

fn split_destroy_timing(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<DelayedDestroyTimingShape>) {
    if let Some((core, timing)) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || delayed_destroy_timing)
    {
        (trim_lexed_commas(core), Some(timing))
    } else {
        (trim_lexed_commas(tokens), None)
    }
}

fn has_unsupported_delayed_timing(tokens: &[OwnedLexToken]) -> bool {
    primitives::has_phrase(tokens, &["end", "of", "combat"])
        || (primitives::contains_word(tokens, "beginning")
            && primitives::contains_word(tokens, "end"))
}

fn has_combat_history_surface(tokens: &[OwnedLexToken]) -> bool {
    (primitives::contains_word(tokens, "dealt")
        && primitives::contains_word(tokens, "damage")
        && primitives::contains_word(tokens, "turn"))
        || [
            &["was", "blocked"][..],
            &["was", "blocking"],
            &["blocking", "it"],
            &["blocked", "it"],
            &["it", "blocked"],
        ]
        .iter()
        .any(|phrase| primitives::has_phrase(tokens, phrase))
}

fn parse_target_combat_history_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyCombatHistoryShape<'_>> {
    if let Some((target_tokens, player_tokens)) = parse_dealt_damage_to_player_filter(tokens) {
        return Some(DestroyCombatHistoryShape::DealtDamageToPlayerThisTurn {
            target_tokens,
            player_tokens,
        });
    }

    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::phrase(&["that", "was", "dealt", "damage", "this", "turn"])
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    (!target_tokens.is_empty())
        .then_some(DestroyCombatHistoryShape::DealtDamageThisTurn { target_tokens })
}

fn parse_dealt_damage_to_player_filter(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (that_idx, (), after_marker) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["that", "dealt", "damage", "to"])
    })?;
    if that_idx == 0 {
        return None;
    }
    let (player_tokens, ()) = primitives::split_lexed_once_before_suffix(after_marker, 1, || {
        primitives::phrase(&["this", "turn"])
    })?;
    let filter_tokens = trim_lexed_commas(&tokens[..that_idx]);
    let player_tokens = trim_lexed_commas(player_tokens);
    (!filter_tokens.is_empty() && !player_tokens.is_empty())
        .then_some((filter_tokens, player_tokens))
}

fn parse_dealt_damage_filter(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (filter_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        alt((
            primitives::phrase(&["that", "was", "dealt", "damage", "this", "turn"]),
            primitives::phrase(&["that", "were", "dealt", "damage", "this", "turn"]),
        ))
    })?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

fn attached_filter_tail_word(word: &str) -> bool {
    matches!(word, "that" | "were" | "was" | "is" | "are")
}

fn attached_target_supported(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("target")).is_some()
        || exact_tokens(tokens, &["you"])
        || exact_tokens(tokens, &["it"])
        || [
            &["that", "creature"][..],
            &["that", "permanent"],
            &["that", "land"],
            &["that", "artifact"],
            &["that", "enchantment"],
        ]
        .iter()
        .any(|prefix| primitives::parse_prefix(tokens, primitives::phrase(prefix)).is_some())
}

fn attached_target_has_timing(tokens: &[OwnedLexToken]) -> bool {
    ["at", "beginning", "end", "combat", "turn", "step", "until"]
        .iter()
        .any(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

fn parse_attached_destroy_all_shape(tokens: &[OwnedLexToken]) -> Option<DestroyAllShape<'_>> {
    let (attached_idx, (), target_tokens) =
        primitives::find_prefix(tokens, || primitives::phrase(&["attached", "to"]))?;
    if attached_idx == 0 {
        return None;
    }
    let mut filter_end = attached_idx;
    while filter_end > 0
        && tokens[filter_end - 1]
            .as_word()
            .is_some_and(attached_filter_tail_word)
    {
        filter_end -= 1;
    }
    let filter_tokens = trim_lexed_commas(&tokens[..filter_end]);
    let target_tokens = trim_lexed_commas(target_tokens);
    if filter_tokens.is_empty()
        || target_tokens.is_empty()
        || !attached_target_supported(target_tokens)
        || attached_target_has_timing(target_tokens)
    {
        return None;
    }
    Some(DestroyAllShape::AttachedTo {
        filter_tokens,
        target_tokens,
    })
}

fn color_choice_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["of", "the", "color", "of", "your", "choice"]),
        primitives::phrase(&["of", "the", "color", "of", "their", "choice"]),
        primitives::phrase(&["of", "color", "of", "your", "choice"]),
        primitives::phrase(&["of", "color", "of", "their", "choice"]),
    ))
    .parse_next(input)
}

fn chosen_this_way_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((
            primitives::phrase(&["chosen", "this", "way"]),
            primitives::phrase(&["that", "were", "chosen", "this", "way"]),
            primitives::phrase(&["that", "was", "chosen", "this", "way"]),
        )),
        opt(alt((
            primitives::phrase(&["by", "any", "player"]),
            primitives::phrase(&["by", "a", "player"]),
        ))),
    )
        .void()
        .parse_next(input)
}

fn strip_negated_chosen_copula(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    const SUFFIXES: &[&[&str]] = &[
        &["that", "werent"],
        &["that", "weren't"],
        &["that", "were", "not"],
        &["that", "wasnt"],
        &["that", "wasn't"],
        &["that", "was", "not"],
        &["werent"],
        &["weren't"],
        &["were", "not"],
        &["wasnt"],
        &["wasn't"],
        &["was", "not"],
        &["not"],
    ];
    let words = crate::lexer::parser_token_word_refs(tokens);
    let mut matched_suffix = None;
    for suffix in SUFFIXES {
        if crate::word_primitives::parse_sequence_suffix(&words, suffix) {
            matched_suffix = Some(*suffix);
            break;
        }
    }
    let suffix = matched_suffix?;
    Some(&tokens[..tokens.len().saturating_sub(suffix.len())])
}

#[cfg(test)]
#[path = "remove_destroy_shapes/tests.rs"]
mod tests;

#[path = "remove_destroy_shapes/counter.rs"]
mod counter_programs;
pub use counter_programs::parse_destroy_counter_constraint_shape;
#[path = "remove_destroy_shapes/core.rs"]
mod core_programs;
use core_programs::parse_destroy_all_shape;
pub use core_programs::parse_destroy_clause_shape;
#[path = "remove_destroy_shapes/reference.rs"]
mod reference_programs;
use reference_programs::{
    has_multi_target_tail, parse_destroy_target_and_attached_shape,
    parse_inline_no_regeneration_target, target_count_before_target,
};
#[path = "remove_destroy_shapes/condition.rs"]
mod condition_programs;
use condition_programs::parse_conditional_destroy_shape;
#[path = "remove_destroy_shapes/combat.rs"]
mod combat_programs;
use combat_programs::has_trailing_attack_or_block_restriction;
