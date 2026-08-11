use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::{conditions, leaf, permission_shapes, primitives, values};
use super::unless_clause::{self, UnlessPaysShape};
use crate::effect::Value;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoveShapeError {
    MissingAmount,
    MissingCounterKeyword,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RemoveCounterDestination<'a> {
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
pub(crate) enum RemoveClauseShape<'a> {
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
pub(crate) enum DelayedDestroyTimingShape {
    EndOfCombat,
    NextEndStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaggedDestroyRelation {
    Matching,
    ExceptMatching,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DestroyAllShape<'a> {
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
pub(crate) enum DestroyCombatHistoryShape<'a> {
    DealtDamageThisTurn {
        target_tokens: &'a [OwnedLexToken],
    },
    DealtDamageToPlayerThisTurn {
        target_tokens: &'a [OwnedLexToken],
        player_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DestroyTargetAndAttachedShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) attachment_filter_tokens: &'a [OwnedLexToken],
    pub(crate) demonstrative_antecedent: Option<ironsmith_core::DemonstrativeAntecedentSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DestroyClauseKind<'a> {
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
pub(crate) struct DestroyClauseShape<'a> {
    pub(crate) timing: Option<DelayedDestroyTimingShape>,
    pub(crate) kind: DestroyClauseKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestroyCounterConstraintKind {
    With,
    Without,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DestroyCounterConstraintShape<'a> {
    pub(crate) base_tokens: &'a [OwnedLexToken],
    pub(crate) constraint_tokens: &'a [OwnedLexToken],
    pub(crate) kind: DestroyCounterConstraintKind,
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

pub(crate) fn parse_remove_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<RemoveClauseShape<'_>, RemoveShapeError> {
    let tokens = trim_shape_edges(tokens);
    if exact_tokens(tokens, &["all", "of", "them"]) {
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
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let suffix = SUFFIXES.iter().find(|suffix| {
        words
            .get(words.len().saturating_sub(suffix.len())..)
            .is_some_and(|tail| tail.iter().copied().eq(suffix.iter().copied()))
    })?;
    Some(&tokens[..tokens.len().saturating_sub(suffix.len())])
}

fn parse_destroy_all_shape(tokens: &[OwnedLexToken]) -> DestroyAllShape<'_> {
    if let Some((filter_tokens, player_tokens)) = parse_dealt_damage_to_player_filter(tokens) {
        return DestroyAllShape::DealtDamageToPlayerThisTurn {
            filter_tokens,
            player_tokens,
        };
    }
    if let Some(filter_tokens) = parse_dealt_damage_filter(tokens) {
        return DestroyAllShape::DealtDamageThisTurn { filter_tokens };
    }
    if let Some(shape) = parse_attached_destroy_all_shape(tokens) {
        return shape;
    }
    if let Some((except_idx, (), exception_tokens)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["except", "for"]))
        && except_idx > 0
    {
        let filter_tokens = trim_lexed_commas(&tokens[..except_idx]);
        let exception_tokens = trim_lexed_commas(exception_tokens);
        let exception_words =
            crate::runtime_backend::lexer::parser_token_word_refs(exception_tokens);
        let attack_eligibility_exception = matches!(
            exception_words.as_slice(),
            [
                "creature" | "creatures",
                "that",
                "couldnt" | "couldn't",
                "attack"
            ]
        );
        if !attack_eligibility_exception
            && !filter_tokens.is_empty()
            && !exception_tokens.is_empty()
        {
            return DestroyAllShape::ExceptFor {
                filter_tokens,
                exception_tokens,
            };
        }
    }
    if let Some((filter_tokens, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || color_choice_suffix)
    {
        let filter_tokens = trim_lexed_commas(filter_tokens);
        if !filter_tokens.is_empty() {
            return DestroyAllShape::ChosenColor { filter_tokens };
        }
    }
    if let Some((base_tokens, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 0, || chosen_this_way_suffix)
    {
        let mut base_tokens = trim_lexed_commas(base_tokens);
        let base_words = crate::runtime_backend::lexer::parser_token_word_refs(base_tokens);
        // In "creatures that aren't of a type chosen this way", `chosen this
        // way` modifies the creature type, not the creatures themselves. Keep
        // the complete phrase in the ordinary object filter so its typed
        // chosen-type exclusion survives. The result-tag route below is for
        // objects that were themselves chosen this way.
        if base_words.ends_with(&["of", "a", "type"])
            || base_words.ends_with(&["of", "the", "type"])
        {
            return DestroyAllShape::Plain {
                filter_tokens: tokens,
            };
        }
        // "not chosen this way" is the complement of the accumulated chosen
        // set. Keep the negation out of the object filter and preserve it as
        // the typed tagged-set relation.
        if let Some(positive_base) = strip_negated_chosen_copula(base_tokens) {
            base_tokens = trim_lexed_commas(positive_base);
            return DestroyAllShape::ChosenThisWay {
                filter_tokens: base_tokens,
                relation: TaggedDestroyRelation::ExceptMatching,
            };
        }
        if let Some((except_idx, (), _)) =
            primitives::find_prefix(base_tokens, || primitives::kw("except").void())
            && except_idx > 0
        {
            let filter_tokens = trim_lexed_commas(&base_tokens[..except_idx]);
            if !filter_tokens.is_empty() {
                return DestroyAllShape::ChosenThisWay {
                    filter_tokens,
                    relation: TaggedDestroyRelation::ExceptMatching,
                };
            }
        }
        return DestroyAllShape::ChosenThisWay {
            filter_tokens: base_tokens,
            relation: TaggedDestroyRelation::Matching,
        };
    }
    DestroyAllShape::Plain {
        filter_tokens: tokens,
    }
}

fn has_trailing_attack_or_block_restriction(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), after_cant)) = primitives::find_prefix(tokens, || {
        alt((
            primitives::kw("cant").void(),
            primitives::kw("cannot").void(),
            primitives::phrase(&["can", "t"]),
        ))
    }) else {
        return false;
    };
    ["attack", "attacks", "block", "blocks"]
        .iter()
        .any(|word| primitives::find_prefix(after_cant, || primitives::kw(word)).is_some())
        && primitives::has_phrase(after_cant, &["this", "turn"])
}

fn target_count_before_target<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        leaf::parse_leaf_target_count_range_prefix_lexed.void(),
        leaf::parse_leaf_choice_count_prefix_lexed.void(),
    ))
    .parse_next(input)?;
    alt((
        primitives::kw("target").void(),
        primitives::kw("targets").void(),
    ))
    .parse_next(input)
}

fn has_multi_target_tail(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), tail)) = primitives::find_prefix(tokens, || primitives::kw("and").void())
    else {
        return false;
    };
    primitives::parse_prefix(tail, primitives::kw("target")).is_some()
        || primitives::parse_prefix(tail, target_count_before_target).is_some()
}

fn parse_destroy_target_and_attached_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyTargetAndAttachedShape<'_>> {
    let (target_tokens, attached_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("and").void())?;
    let target_tokens = trim_lexed_commas(target_tokens);
    let target_starts_with_selection =
        primitives::parse_prefix(target_tokens, primitives::kw("target").void()).is_some()
            || primitives::parse_prefix(target_tokens, target_count_before_target).is_some();
    if !target_starts_with_selection {
        return None;
    }

    let ((), attached_tokens) =
        primitives::parse_prefix(attached_tokens, primitives::kw("all").void())?;
    let (attachment_filter_tokens, attachment_reference_tokens) =
        primitives::split_lexed_once_on_separator(attached_tokens, || {
            primitives::phrase(&["attached", "to"]).void()
        })?;
    let attachment_filter_tokens = trim_lexed_commas(attachment_filter_tokens);
    let attachment_reference_tokens = trim_lexed_commas(attachment_reference_tokens);
    if attachment_filter_tokens.is_empty() {
        return None;
    }

    let demonstrative_antecedent = if exact_tokens(attachment_reference_tokens, &["it"])
        || exact_tokens(attachment_reference_tokens, &["them"])
    {
        None
    } else {
        let [that, noun] = attachment_reference_tokens else {
            return None;
        };
        if !that.is_word("that") {
            return None;
        }
        Some(ironsmith_core::DemonstrativeAntecedentSurface::from_noun(
            noun.as_word()?,
        )?)
    };

    Some(DestroyTargetAndAttachedShape {
        target_tokens,
        attachment_filter_tokens,
        demonstrative_antecedent,
    })
}

fn parse_conditional_destroy_shape(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (if_idx, (), predicate_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("if").void())?;
    let mut target_tokens = trim_lexed_commas(&tokens[..if_idx]);
    while let Some((head, ())) =
        primitives::split_lexed_once_before_suffix(target_tokens, 0, || {
            primitives::kw("instead").void()
        })
    {
        target_tokens = trim_lexed_commas(head);
    }
    Some((target_tokens, trim_lexed_commas(predicate_tokens)))
}

fn parse_inline_no_regeneration_target(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::kw("and"),
            primitives::kw("it"),
            alt((
                primitives::kw("cant").void(),
                primitives::kw("can't").void(),
                primitives::kw("cannot").void(),
            )),
            primitives::kw("be"),
            primitives::kw("regenerated"),
        )
            .void()
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    (!target_tokens.is_empty()).then_some(target_tokens)
}

pub(crate) fn parse_destroy_clause_shape(tokens: &[OwnedLexToken]) -> DestroyClauseShape<'_> {
    let tokens = trim_shape_edges(tokens);
    let (core_tokens, timing) = split_destroy_timing(tokens);
    let kind = if core_tokens.is_empty() {
        DestroyClauseKind::Empty
    } else if timing.is_none() && has_unsupported_delayed_timing(tokens) {
        DestroyClauseKind::UnsupportedDelayedTiming
    } else if let Some(((), all_tokens)) = primitives::parse_prefix(core_tokens, all_or_each_word) {
        let all_shape = parse_destroy_all_shape(trim_lexed_commas(all_tokens));
        if matches!(
            all_shape,
            DestroyAllShape::DealtDamageThisTurn { .. }
                | DestroyAllShape::DealtDamageToPlayerThisTurn { .. }
        ) || !has_combat_history_surface(core_tokens)
        {
            DestroyClauseKind::All(all_shape)
        } else {
            DestroyClauseKind::UnsupportedCombatHistory
        }
    } else if let Some(combat) = parse_target_combat_history_shape(core_tokens) {
        DestroyClauseKind::CombatHistory(combat)
    } else if has_combat_history_surface(core_tokens) {
        DestroyClauseKind::UnsupportedCombatHistory
    } else if let Some((target_tokens, unless_tokens)) =
        primitives::split_lexed_once_on_separator(core_tokens, || primitives::kw("unless").void())
    {
        let target_tokens = trim_lexed_commas(target_tokens);
        if let Some(predicate) = conditions::parse_target_set_predicate(unless_tokens)
            && !target_tokens.is_empty()
        {
            DestroyClauseKind::UnlessTargetSetPredicate {
                target_tokens,
                predicate,
            }
        } else {
            match unless_clause::parse_unless_pays_shape_tokens(unless_tokens) {
                Some(payment) if !target_tokens.is_empty() => DestroyClauseKind::UnlessPays {
                    target_tokens,
                    payment,
                },
                _ => DestroyClauseKind::UnsupportedUnless,
            }
        }
    } else if has_trailing_attack_or_block_restriction(core_tokens) {
        DestroyClauseKind::TrailingAttackOrBlockRestriction
    } else if let Some((target_tokens, predicate_tokens)) =
        parse_conditional_destroy_shape(core_tokens)
    {
        if target_tokens.is_empty() || predicate_tokens.is_empty() {
            DestroyClauseKind::UnsupportedConditional
        } else {
            DestroyClauseKind::Conditional {
                target_tokens,
                predicate_tokens,
            }
        }
    } else if let Some(target_tokens) = parse_inline_no_regeneration_target(core_tokens) {
        DestroyClauseKind::InlineNoRegeneration { target_tokens }
    } else if let Some(shape) = parse_destroy_target_and_attached_shape(core_tokens) {
        DestroyClauseKind::TargetAndAttached(shape)
    } else if has_multi_target_tail(core_tokens) {
        DestroyClauseKind::MultiTarget
    } else if primitives::parse_prefix(core_tokens, primitives::phrase(&["target", "blocked"]))
        .is_some()
    {
        DestroyClauseKind::Blocked {
            target_tokens: core_tokens.to_vec(),
        }
    } else {
        DestroyClauseKind::Plain {
            target_tokens: core_tokens,
        }
    };
    DestroyClauseShape { timing, kind }
}

pub(crate) fn parse_destroy_counter_constraint_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyCounterConstraintShape<'_>> {
    let (with_idx, (), tail) = primitives::find_prefix(tokens, || primitives::kw("with").void())?;
    let base_tokens = trim_lexed_commas(&tokens[..with_idx]);
    if base_tokens.is_empty() {
        return None;
    }
    if let Some(((), constraint_tokens)) =
        primitives::parse_prefix(tail, primitives::kw("no").void())
    {
        return Some(DestroyCounterConstraintShape {
            base_tokens,
            constraint_tokens: trim_lexed_commas(constraint_tokens),
            kind: DestroyCounterConstraintKind::Without,
        });
    }
    Some(DestroyCounterConstraintShape {
        base_tokens,
        constraint_tokens: trim_lexed_commas(tail),
        kind: DestroyCounterConstraintKind::With,
    })
}

#[cfg(test)]
#[path = "remove_destroy_shapes/tests.rs"]
mod tests;
