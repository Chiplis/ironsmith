use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::filter::StackObjectKind;
use crate::target::ObjectFilter;
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{filters, leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivationChoicePrefix<'a> {
    pub count: ChoiceCount,
    pub rest: &'a [OwnedLexToken],
}

pub fn parse_activation_choice_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivationChoicePrefix<'_>> {
    if tokens.is_empty() {
        return None;
    }
    if let Some((count, rest)) =
        primitives::parse_prefix(tokens, leaf::parse_leaf_choice_count_prefix_lexed)
    {
        return Some(ActivationChoicePrefix { count, rest });
    }
    Some(ActivationChoicePrefix {
        count: ChoiceCount::exactly(1),
        rest: tokens,
    })
}

pub fn parse_activation_exile_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let base_len = primitives::parse_all(
        tokens,
        parse_single_graveyard_suffix_lexed,
        "activation-exile-single-graveyard-filter",
    )
    .ok();
    let mut filter = if let Some(base_len) = base_len {
        let mut filter =
            filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[..base_len], false)?;
        filter.zone = Some(Zone::Graveyard);
        filter.single_graveyard = true;
        filter
    } else {
        filters::parse_object_filter_with_grammar_entrypoint_lexed(tokens, false)?
    };
    if primitives::TokenWordView::new(tokens)
        .word_refs()
        .iter()
        .any(|word| matches!(*word, "spell" | "spells"))
    {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(StackObjectKind::Spell);
        filter.has_mana_cost = true;
    }
    Ok(filter)
}

fn parse_single_graveyard_suffix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let mut prefix_len = 0usize;
    loop {
        let mut suffix = input.clone();
        if primitives::phrase(&["from", "a", "single", "graveyard"])
            .parse_next(&mut suffix)
            .is_ok()
            && suffix.peek_token().is_none()
        {
            *input = suffix;
            return Ok(prefix_len);
        }
        any.parse_next(input)?;
        prefix_len += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn choice_prefix_and_exile_scope_are_typed() {
        let tokens = lex_line("one or more artifact cards", 0).unwrap();
        let parsed = parse_activation_choice_prefix_tokens(&tokens).unwrap();
        assert_eq!(parsed.count, ChoiceCount::at_least(1));
        assert_eq!(render_token_slice(parsed.rest), "artifact cards");

        let tokens = lex_line("artifact card from a single graveyard", 0).unwrap();
        let filter = parse_activation_exile_filter_tokens(&tokens).unwrap();
        assert_eq!(filter.card_types, vec![crate::types::CardType::Artifact]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.single_graveyard);
    }
}
