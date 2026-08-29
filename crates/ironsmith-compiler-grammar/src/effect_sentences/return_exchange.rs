use super::*;
use crate::cards::builders::SubjectVerbActionAst;
use crate::effect_sentences::SubjectVerbPrimitiveClause;
fn parse_return_back_reference_target(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(reference) = crate::grammar::effects::parse_return_back_reference_shape(tokens) {
        let span = span_from_tokens(tokens);
        if reference == crate::grammar::effects::ReturnBackReferenceShape::Them {
            let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
            filter.set_plural_pronoun_reference_surface(true);
            return Ok(TargetAst::Object(filter, None, span));
        }
        if reference == crate::grammar::effects::ReturnBackReferenceShape::Demonstrative {
            let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
            filter.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                crate::lexer::render_token_slice(tokens).trim().to_string(),
            ));
            return Ok(TargetAst::Object(filter, None, span));
        }
        Ok(TargetAst::Tagged(TagKey::from(IT_TAG), span))
    } else {
        parse_target_phrase(tokens)
    }
}

fn set_return_destination_first_surface(target: &mut TargetAst, destination_first: bool) {
    match target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter.set_return_destination_first_surface(destination_first);
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            set_return_destination_first_surface(inner, destination_first);
        }
        _ => {}
    }
}

fn strip_except_this_card_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    if tokens.len() >= 3
        && tokens[tokens.len() - 3].is_word("except")
        && tokens[tokens.len() - 2].is_word("this")
        && tokens[tokens.len() - 1].is_word("card")
    {
        (&tokens[..tokens.len() - 3], true)
    } else {
        (tokens, false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    crate::grammar::effects::parse_return_timing_words_shape(words).map(|shape| match shape {
        crate::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
            DelayedReturnTimingAst::NextEndStep(player)
        }
        crate::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
            DelayedReturnTimingAst::NextUpkeep(player)
        }
        crate::grammar::effects::ReturnTimingShape::EndOfCombat => {
            DelayedReturnTimingAst::EndOfCombat
        }
    })
}
pub fn wrap_return_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedReturnTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
    }
}

#[cfg(test)]
#[path = "return_exchange_inline_tests.rs"]
mod tests;

#[path = "return_exchange/return_exchange_core_programs.rs"]
mod return_exchange_core_programs;
pub use return_exchange_core_programs::parse_exchange;
#[path = "return_exchange/return_exchange_zone_programs.rs"]
mod return_exchange_zone_programs;
pub use return_exchange_zone_programs::parse_return;
