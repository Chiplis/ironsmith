use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::ChoiceCount;
use crate::runtime_backend::front_end::grammar::{filters, leaf, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas,
};
use crate::target::ObjectFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProliferateChoosePhaseOutShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: ObjectFilter,
}

fn commas<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)
}

fn proliferate_then_choose<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("you")).parse_next(input)?;
    primitives::kw("proliferate").parse_next(input)?;
    commas(input)?;
    primitives::phrase(&["then", "choose"])
        .void()
        .parse_next(input)
}

fn counter_received_this_way_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["that", "had"]).parse_next(input)?;
    alt((
        primitives::phrase(&["a", "counter"]),
        primitives::kw("counters").void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["put", "on", "them", "this", "way"])
        .void()
        .parse_next(input)
}

fn chosen_objects_phase_out<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["those", "permanents"]),
        primitives::phrase(&["those", "objects"]),
        primitives::kw("they").void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["phase", "out"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_proliferate_choose_phase_out_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ProliferateChoosePhaseOutShape> {
    let sentences = split_lexed_sentences(tokens);
    let [selection, phase_out] = sentences.as_slice() else {
        return None;
    };
    let (_, selection_tail) = primitives::parse_prefix(selection, proliferate_then_choose)?;
    let (count_and_filter, suffix) =
        primitives::split_lexed_once_on_separator(selection_tail, || {
            counter_received_this_way_suffix
        })?;
    if !trim_lexed_commas(suffix).is_empty() {
        return None;
    }
    let (count, filter_tokens) = primitives::parse_prefix(
        trim_lexed_commas(count_and_filter),
        leaf::parse_leaf_choice_count_prefix_lexed,
    )?;
    let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(filter_tokens),
        false,
    )
    .ok()?;
    filter.zone = Some(Zone::Battlefield);
    primitives::parse_all_or_none(
        trim_lexed_commas(phase_out),
        chosen_objects_phase_out,
        "chosen objects phase out",
    )
    .ok()
    .flatten()?;
    Some(ProliferateChoosePhaseOutShape { count, filter })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::types::CardType;

    #[test]
    fn parses_proliferate_choose_phase_out_for_typed_filter() {
        let tokens = lex_line(
            "Proliferate, then choose up to two artifacts you control that had a counter put on them this way. Those permanents phase out.",
            0,
        )
        .unwrap();
        let shape = parse_proliferate_choose_phase_out_tokens(&tokens).unwrap();
        assert_eq!(shape.count, ChoiceCount::up_to(2));
        assert!(shape.filter.card_types.contains(&CardType::Artifact));
    }
}
