//! Balance's procedure and its "the same way" re-applications (Balance,
//! Balancing Act, Magus of the Balance, Restore Balance).
//!
//! "Each player chooses a number of lands they control equal to the number of
//! lands controlled by the player who controls the fewest, then sacrifices the
//! rest." followed by "Players discard cards and sacrifice creatures the same
//! way." The second sentence has no procedure of its own: it repeats the first
//! one, in the order written, over each new object domain (cards in hand are
//! discarded rather than sacrificed). Each repetition is a complete
//! choose-then-rest step over its own domain, counted when that step begins.
use super::*;
use crate::cards::builders::{ForEachEffectAst, ObjectChoiceEffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::ObjectFilter as Filter;
use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::grammar::primitives;

type Stream<'a> = crate::lexer::LexStream<'a>;
type PResult<T> = Result<T, ErrMode<ContextError>>;

/// One object domain the procedure runs over.
#[derive(Debug, Clone, PartialEq)]
enum Domain {
    /// Permanents of this description each player controls; the rest are
    /// sacrificed.
    Battlefield(Filter),
    /// Cards in each player's hand; the rest are discarded.
    Hand,
}

fn permanent_noun(input: &mut Stream<'_>) -> PResult<Filter> {
    alt((
        primitives::kw("lands").map(|_| Filter::land()),
        primitives::kw("creatures").map(|_| Filter::creature()),
        primitives::kw("permanents").map(|_| Filter::permanent()),
    ))
    .parse_next(input)
}

/// "each player chooses a number of <X> they control equal to the number of
/// <X> controlled by the player who controls the fewest, then sacrifices the
/// rest"
fn balance_head(input: &mut Stream<'_>) -> PResult<Filter> {
    primitives::phrase(&["each", "player", "chooses", "a", "number", "of"]).parse_next(input)?;
    let chosen = permanent_noun.parse_next(input)?;
    primitives::phrase(&["they", "control", "equal", "to", "the", "number", "of"])
        .parse_next(input)?;
    let counted = permanent_noun.parse_next(input)?;
    primitives::phrase(&[
        "controlled", "by", "the", "player", "who", "controls", "the", "fewest",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "sacrifices", "the", "rest"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if chosen != counted {
        return Err(primitives::backtrack_err("balance", "one object description"));
    }
    Ok(chosen)
}

fn same_way_action(input: &mut Stream<'_>) -> PResult<Domain> {
    alt((
        (
            alt((primitives::kw("discard"), primitives::kw("discards"))),
            primitives::kw("cards"),
        )
            .map(|_| Domain::Hand),
        (
            alt((primitives::kw("sacrifice"), primitives::kw("sacrifices"))),
            permanent_noun,
        )
            .map(|(_, filter)| Domain::Battlefield(filter)),
    ))
    .parse_next(input)
}

/// "<each player|players> <action> [and <action>] the same way"
fn same_way_tail(input: &mut Stream<'_>) -> PResult<Vec<Domain>> {
    alt((
        primitives::phrase(&["each", "player"]),
        primitives::kw("players").void(),
    ))
    .parse_next(input)?;
    let first = same_way_action.parse_next(input)?;
    let second = opt((primitives::kw("and"), same_way_action).map(|(_, domain)| domain))
        .parse_next(input)?;
    primitives::phrase(&["the", "same", "way"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(std::iter::once(first).chain(second).collect())
}

/// One choose-then-rest step: each player, in turn, chooses as many objects
/// of the domain as the player with the fewest has (CR 101.4: in APNAP
/// order), then sacrifices or discards the rest of that domain.
fn balance_step(domain: &Domain, tag: crate::tag::TagRef) -> EffectAst {
    let (chosen, counted) = match domain {
        Domain::Battlefield(filter) => {
            let filter = filter.clone().in_zone(Zone::Battlefield);
            (
                filter.clone().controlled_by(PlayerFilter::IteratedPlayer),
                filter.controlled_by(PlayerFilter::Any),
            )
        }
        Domain::Hand => (
            Filter::default().in_zone(Zone::Hand).owned_by(PlayerFilter::IteratedPlayer),
            Filter::default().in_zone(Zone::Hand).owned_by(PlayerFilter::Any),
        ),
    };
    let rest = chosen.clone().not_tagged(tag.clone());
    let rest_action = match domain {
        Domain::Battlefield(_) => EffectAst::subject_verb_sacrifice_all(PlayerAst::That, rest),
        Domain::Hand => EffectAst::subject_verb_discard(
            PlayerAst::That,
            Value::Count(rest.clone()),
            false,
            false,
            Some(rest),
            None,
        ),
    };
    EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: chosen,
                count: ChoiceCount::dynamic_x(),
                count_value: Some(Value::LeastCount(counted)),
                player: PlayerAst::That,
                tag,
            }),
            rest_action,
        ],
    })
}

fn step_tag(tokens: &[OwnedLexToken], step: usize) -> crate::tag::TagRef {
    let prefix = match step {
        0 => "balance_keep_first",
        1 => "balance_keep_second",
        _ => "balance_keep_third",
    };
    crate::util::helper_tag_for_tokens(tokens, prefix)
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let Some(head) = primitives::probe_all(first.lowered(), balance_head, "balance procedure")
    else {
        return Ok(None);
    };
    let Some(repeats) =
        primitives::probe_all(second.lowered(), same_way_tail, "the same way repetition")
    else {
        return Ok(None);
    };
    let mut domains = vec![Domain::Battlefield(head)];
    domains.extend(repeats);
    let effects = domains
        .iter()
        .enumerate()
        .map(|(step, domain)| {
            let tokens = if step == 0 { first.lowered() } else { second.lowered() };
            balance_step(domain, step_tag(tokens, step))
        })
        .collect();
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentence(text: &str) -> SentenceInput {
        SentenceInput::from_lexed(&crate::lexer::lex_line(text, 0).unwrap())
    }

    #[test]
    fn the_same_way_repeats_the_procedure_over_each_domain_in_order() {
        let sentences = [
            sentence(
                "Each player chooses a number of lands they control equal to the number of lands controlled by the player who controls the fewest, then sacrifices the rest",
            ),
            sentence("Players discard cards and sacrifice creatures the same way"),
        ];
        let effects = read(&sentences, 0).unwrap().expect("balance procedure");
        assert_eq!(effects.len(), 3);
        let debug = format!("{effects:?}");
        assert_eq!(debug.matches("LeastCount").count(), 3, "{debug}");
        assert!(debug.contains("SacrificeAll"), "{debug}");
        assert!(debug.contains("Hand"), "{debug}");
        let unrelated = [
            sentence(
                "Each player chooses a number of lands they control equal to the number of lands controlled by the player who controls the fewest, then sacrifices the rest",
            ),
            sentence("Players discard cards"),
        ];
        assert!(read(&unrelated, 0).unwrap().is_none());
    }
}
