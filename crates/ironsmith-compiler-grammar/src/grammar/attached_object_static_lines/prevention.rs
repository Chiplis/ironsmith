use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::object::CounterType;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, parser_token_word_refs, trim_lexed_commas,
};
use super::super::{filters, leaf, primitives};
use super::subjects::{semantic_finish, semantic_kw, semantic_phrase};

#[path = "prevention/prevent_all.rs"]
mod prevent_all;
pub use prevent_all::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveCounterPreventionAmount {
    Fixed(u32),
    DamageAmount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveCounterPreventionFollowUp {
    pub counter_type: CounterType,
    pub counters_per_removed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveCounterPreventionSpec<'a> {
    pub counter_type: CounterType,
    pub amount: RemoveCounterPreventionAmount,
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
    pub follow_up: Option<RemoveCounterPreventionFollowUp>,
    pub one_damage_per_counter: bool,
    pub separate_removal_sentence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutCounterPreventionSpec<'a> {
    General {
        condition_tokens: Option<&'a [OwnedLexToken]>,
        display_prefix_tokens: &'a [OwnedLexToken],
        effect_tokens: &'a [OwnedLexToken],
    },
    Noncombat,
    CreatureCombat,
}

pub fn parse_remove_counter_prevention_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RemoveCounterPreventionSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_remove_counter_prevention_lexed,
        "remove-counter damage prevention",
    )
}

pub fn parse_put_counter_prevention_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PutCounterPreventionSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_general_put_counter_prevention_lexed,
            parse_noncombat_put_counter_prevention_lexed,
            parse_creature_combat_put_counter_prevention_lexed,
        )),
        "put-counter damage prevention",
    )
}

fn parse_remove_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RemoveCounterPreventionSpec<'a>> {
    alt((
        parse_one_damage_per_counter_prevention_lexed,
        parse_standard_remove_counter_prevention_lexed,
    ))
    .parse_next(input)
}

fn parse_standard_remove_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RemoveCounterPreventionSpec<'a>> {
    semantic_phrase(&["if", "damage", "would", "be", "dealt", "to"]).parse_next(input)?;
    parse_this_source(input)?;
    let condition_tokens = if peek(semantic_kw("while")).parse_next(input).is_ok() {
        semantic_kw("while").parse_next(input)?;
        let condition_tokens =
            repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(semantic_kw("prevent")))
                .map(|((), _)| ())
                .take()
                .parse_next(input)?;
        Some(trim_lexed_commas(condition_tokens))
    } else {
        None
    };
    semantic_phrase(&["prevent", "that", "damage"]).parse_next(input)?;
    let conjoined = opt(semantic_kw("and")).parse_next(input)?.is_some();
    semantic_kw("remove").parse_next(input)?;
    let amount = opt(alt((
        semantic_phrase(&["that", "many"]).value(RemoveCounterPreventionAmount::DamageAmount),
        leaf::parse_leaf_number_prefix_lexed.map(RemoveCounterPreventionAmount::Fixed),
        alt((semantic_kw("a"), semantic_kw("an"))).value(RemoveCounterPreventionAmount::Fixed(1)),
    )))
    .map(|amount| amount.unwrap_or(RemoveCounterPreventionAmount::Fixed(1)))
    .parse_next(input)?;
    let (_, descriptor) = (
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((semantic_kw("counter"), semantic_kw("counters")))),
        )
        .void(),
        alt((semantic_kw("counter"), semantic_kw("counters"))),
    )
        .with_taken()
        .parse_next(input)?;
    semantic_kw("from").parse_next(input)?;
    alt((parse_this_source, semantic_kw("it"))).parse_next(input)?;
    let counter_type = filters::parse_counter_type_from_tokens(trim_lexed_commas(descriptor))
        .ok_or_else(|| {
            primitives::backtrack_err("remove-counter prevention", "known counter type")
        })?;
    let follow_up = opt(parse_each_player_counter_follow_up_lexed).parse_next(input)?;
    if follow_up.is_some_and(|follow_up| follow_up.removed_counter_type != counter_type) {
        return Err(primitives::backtrack_err(
            "remove-counter prevention follow-up",
            "the counter type removed by the preceding action",
        ));
    }
    semantic_finish(input)?;
    Ok(RemoveCounterPreventionSpec {
        counter_type,
        amount,
        condition_tokens,
        follow_up: follow_up.map(|follow_up| RemoveCounterPreventionFollowUp {
            counter_type: follow_up.counter_type,
            counters_per_removed: follow_up.counters_per_removed,
        }),
        one_damage_per_counter: false,
        separate_removal_sentence: !conjoined,
    })
}

fn parse_counter_type_before_counter_noun<'a>(
    input: &mut LexStream<'a>,
    context: &'static str,
) -> WResult<CounterType> {
    let (_, descriptor) = (
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((semantic_kw("counter"), semantic_kw("counters")))),
        )
        .void(),
        alt((semantic_kw("counter"), semantic_kw("counters"))),
    )
        .with_taken()
        .parse_next(input)?;
    filters::parse_counter_type_from_tokens(trim_lexed_commas(descriptor))
        .ok_or_else(|| primitives::backtrack_err(context, "known counter type"))
}

fn parse_one_damage_per_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RemoveCounterPreventionSpec<'a>> {
    semantic_phrase(&[
        "for", "each", "1", "damage", "that", "would", "be", "dealt", "to",
    ])
    .parse_next(input)?;
    parse_this_source(input)?;
    opt(primitives::comma()).parse_next(input)?;
    semantic_kw("if").parse_next(input)?;
    let (condition_counter_type, condition_tokens) = (
        semantic_phrase(&["it", "has"]).void(),
        alt((semantic_kw("a"), semantic_kw("an"))).void(),
        |input: &mut LexStream<'a>| {
            parse_counter_type_before_counter_noun(input, "per-damage counter condition")
        },
        semantic_phrase(&["on", "it"]).void(),
    )
        .map(|(_, _, counter_type, _)| counter_type)
        .with_taken()
        .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    semantic_kw("remove").parse_next(input)?;
    alt((semantic_kw("a"), semantic_kw("an"))).parse_next(input)?;
    let removed_counter_type =
        parse_counter_type_before_counter_noun(input, "per-damage counter removal")?;
    if condition_counter_type != removed_counter_type {
        return Err(primitives::backtrack_err(
            "per-damage counter prevention",
            "the counter type named by its condition",
        ));
    }
    semantic_phrase(&["from", "it", "and", "prevent", "that", "1", "damage"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(RemoveCounterPreventionSpec {
        counter_type: removed_counter_type,
        amount: RemoveCounterPreventionAmount::DamageAmount,
        condition_tokens: Some(trim_lexed_commas(condition_tokens)),
        follow_up: None,
        one_damage_per_counter: true,
        separate_removal_sentence: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedEachPlayerCounterFollowUp {
    counter_type: CounterType,
    counters_per_removed: u32,
    removed_counter_type: CounterType,
}

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((semantic_kw("counter"), semantic_kw("counters")))
        .void()
        .parse_next(input)
}

fn fixed_counter_amount<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    alt((
        leaf::parse_leaf_number_prefix_lexed,
        alt((semantic_kw("a"), semantic_kw("an"))).value(1),
    ))
    .parse_next(input)
}

fn counter_type_before_noun<'a>(input: &mut LexStream<'a>) -> WResult<CounterType> {
    let (_, descriptor) = (
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(counter_noun)).void(),
        counter_noun,
    )
        .with_taken()
        .parse_next(input)?;
    filters::parse_counter_type_from_tokens(trim_lexed_commas(descriptor))
        .ok_or_else(|| primitives::backtrack_err("player counter follow-up", "known counter type"))
}

fn parse_each_player_counter_follow_up_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ParsedEachPlayerCounterFollowUp> {
    opt(primitives::comma()).parse_next(input)?;
    semantic_kw("then").parse_next(input)?;
    semantic_kw("give").parse_next(input)?;
    semantic_phrase(&["each", "player"]).parse_next(input)?;
    let counters_per_removed = fixed_counter_amount.parse_next(input)?;
    let counter_type = counter_type_before_noun.parse_next(input)?;
    semantic_phrase(&["for", "each"]).parse_next(input)?;
    let removed_counter_type = counter_type_before_noun.parse_next(input)?;
    semantic_phrase(&["removed", "this", "way"]).parse_next(input)?;
    Ok(ParsedEachPlayerCounterFollowUp {
        counter_type,
        counters_per_removed,
        removed_counter_type,
    })
}

#[cfg(test)]
#[path = "prevention/tests.rs"]
mod tests;

#[path = "prevention/reference.rs"]
mod reference_programs;
use reference_programs::{parse_this_source, validate_source_reference};
#[path = "prevention/counter.rs"]
mod counter_programs;
use counter_programs::parse_counter_destination;
#[path = "prevention/combat.rs"]
mod combat_programs;
use combat_programs::{
    parse_creature_combat_put_counter_prevention_lexed, parse_general_put_counter_prevention_lexed,
    parse_noncombat_put_counter_prevention_lexed,
};
