//! "You may pay {0} rather than pay the echo cost for permanents you
//! control." (Thick-Skinned Goblin): an alternative price for the echo
//! upkeep payment of each matching permanent (CR 118.9, 702.30a).
use super::*;

use winnow::combinator::{peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::LexStream;

struct EchoCostAlternativeShape<'a> {
    replacement_cost_tokens: &'a [OwnedLexToken],
    filter_tokens: &'a [OwnedLexToken],
}

fn echo_cost_alternative_shape<'a>(
    input: &mut LexStream<'a>,
) -> winnow::error::ModalResult<EchoCostAlternativeShape<'a>> {
    primitives::phrase(&["you", "may", "pay"]).parse_next(input)?;
    let replacement_cost_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["rather", "than", "pay", "the", "echo", "cost"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["rather", "than", "pay", "the", "echo", "cost", "for"])
        .parse_next(input)?;
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::sentence_end()),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EchoCostAlternativeShape {
        replacement_cost_tokens,
        filter_tokens,
    })
}

pub(super) fn parse_echo_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Ok(shape) = echo_cost_alternative_shape.parse(LexStream::new(tokens)) else {
        return Ok(None);
    };
    let replacement = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(
        shape.replacement_cost_tokens,
    )?;
    if replacement.has_non_mana_costs() {
        return Err(CardTextError::ParseError(format!(
            "unsupported non-mana echo alternative cost (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }
    let mana = replacement
        .mana_cost()
        .cloned()
        .unwrap_or_else(crate::mana::ManaCost::new);
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    let display = format!(
        "You may pay {} rather than pay the echo cost for {}",
        mana.to_oracle(),
        crate::lexer::render_token_slice(shape.filter_tokens).trim()
    );
    Ok(Some(StaticAbility::echo_cost_alternative(filter, mana, display)))
}
