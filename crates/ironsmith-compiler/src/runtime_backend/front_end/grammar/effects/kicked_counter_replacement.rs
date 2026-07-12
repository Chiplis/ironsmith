use winnow::combinator::opt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::{TargetAst, TextSpan};
use crate::effect::Value;
use crate::target::{Comparison, ObjectFilter};

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives, structure};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CounterSpellTargetReference {
    Explicit(TargetAst),
    PriorSpell(Option<TextSpan>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CounterSpellManaValueGate {
    pub(crate) target: CounterSpellTargetReference,
    pub(crate) limit: Value,
    pub(crate) filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KickedCounterReplacementFact {
    pub(crate) base: CounterSpellManaValueGate,
    pub(crate) kicked: CounterSpellManaValueGate,
}

fn mana_value_gate_tail<'a>(input: &mut LexStream<'a>) -> WResult<(Value, ObjectFilter)> {
    primitives::phrase(&["if", "its", "mana", "value", "is"]).parse_next(input)?;
    let limit = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "less"]).parse_next(input)?;

    let limit = i32::try_from(limit)
        .map_err(|_| primitives::backtrack_err("mana value limit", "i32-compatible number"))?;
    let mut filter = ObjectFilter::default();
    filter.mana_value = Some(Comparison::LessThanOrEqual(limit));
    Ok((Value::Fixed(limit), filter))
}

fn parse_base_gate_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterSpellManaValueGate> {
    primitives::kw("counter").parse_next(input)?;
    let target_tokens = (primitives::kw("target"), primitives::kw("spell"))
        .take()
        .parse_next(input)?;
    let (limit, filter) = mana_value_gate_tail.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(CounterSpellManaValueGate {
        target: CounterSpellTargetReference::Explicit(TargetAst::Spell(
            primitives::token_slice_span(target_tokens),
        )),
        limit,
        filter,
    })
}

fn parse_kicked_gate_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterSpellManaValueGate> {
    primitives::phrase(&["if", "this", "spell", "was", "kicked"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("counter").parse_next(input)?;
    let reference_tokens = (primitives::kw("that"), primitives::kw("spell"))
        .take()
        .parse_next(input)?;
    let (limit, filter) = mana_value_gate_tail.parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(CounterSpellManaValueGate {
        target: CounterSpellTargetReference::PriorSpell(primitives::token_slice_span(
            reference_tokens,
        )),
        limit,
        filter,
    })
}

pub(crate) fn parse_kicked_counter_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KickedCounterReplacementFact> {
    let sentences = structure::split_lexed_sentences(tokens);
    let [base_tokens, kicked_tokens] = sentences.as_slice() else {
        return None;
    };
    let base = primitives::parse_all(
        base_tokens,
        parse_base_gate_lexed,
        "counter-spell-mana-value-base",
    )
    .ok()?;
    let kicked = primitives::parse_all(
        kicked_tokens,
        parse_kicked_gate_lexed,
        "kicked-counter-spell-mana-value-replacement",
    )
    .ok()?;
    Some(KickedCounterReplacementFact { base, kicked })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_generic_bounds_and_typed_target_references() {
        let tokens = lex_line(
            "Counter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less instead.",
            0,
        )
        .unwrap();
        let parsed = parse_kicked_counter_replacement_tokens(&tokens).unwrap();

        assert_eq!(parsed.base.limit, Value::Fixed(3));
        assert_eq!(parsed.kicked.limit, Value::Fixed(7));
        assert!(matches!(
            parsed.base.target,
            CounterSpellTargetReference::Explicit(TargetAst::Spell(Some(_)))
        ));
        assert!(matches!(
            parsed.kicked.target,
            CounterSpellTargetReference::PriorSpell(Some(_))
        ));
        assert!(matches!(
            parsed.base.filter.mana_value,
            Some(Comparison::LessThanOrEqual(3))
        ));
        assert!(matches!(
            parsed.kicked.filter.mana_value,
            Some(Comparison::LessThanOrEqual(7))
        ));
    }

    #[test]
    fn rejects_nonreplacement_followup() {
        let tokens = lex_line(
            "Counter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less.",
            0,
        )
        .unwrap();
        assert!(parse_kicked_counter_replacement_tokens(&tokens).is_none());
    }
}
