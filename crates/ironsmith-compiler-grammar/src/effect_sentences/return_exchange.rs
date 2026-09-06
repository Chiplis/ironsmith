use crate::cards::builders::{DelayedEffectAst, TriggerSpec};
use super::*;
use crate::cards::builders::SubjectVerbActionAst;
use crate::effect_sentences::SubjectVerbPrimitiveClause;
pub(crate) fn parse_return_with_event_timing(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(when_idx) = tokens.iter().position(|token| token.is_word("when")) else {
        return Ok(None);
    };
    if when_idx <= 1 { return Ok(None); }
    let trigger_words = crate::lexer::token_word_refs(&tokens[when_idx..]);
    let trigger = if trigger_words.len() == 4 && trigger_words[1] == "that"
        && trigger_words[3] == "dies"
    {
        let mut filter = crate::object_filters::parse_object_filter_lexed(
            &tokens[when_idx + 2..tokens.len() - 1], false,
        )?;
        filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::It.bind().into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
        TriggerSpec::Dies(filter)
    } else {
        crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens[when_idx..])?
    };
    let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens[..when_idx])?;
    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
        trigger,
        effects,
        one_shot: true,
        duration: Until::Forever,
        either_of_watched_objects: false,
        while_any_tagged_object_in_zone: None,
    })]))
}

fn parse_return_back_reference_target(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(reference) = crate::grammar::effects::parse_return_back_reference_shape(tokens) {
        let span = span_from_tokens(tokens);
        if reference == crate::grammar::effects::ReturnBackReferenceShape::Them {
            let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
            filter.set_plural_pronoun_reference_surface(true);
            return Ok(TargetAst::Object(filter, None, span));
        }
        if reference == crate::grammar::effects::ReturnBackReferenceShape::Demonstrative {
            let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
            filter.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                crate::lexer::render_token_slice(tokens).trim().to_string(),
            ));
            return Ok(TargetAst::Object(filter, None, span));
        }
        Ok(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span,
        ))
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
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        }),
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        }),
        DelayedReturnTimingAst::EndOfCombat => EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        }),
    }
}

#[cfg(test)]
#[path = "return_exchange_inline_tests.rs"]
mod tests;

#[path = "return_exchange/return_exchange_core.rs"]
mod return_exchange_core_programs;
pub use return_exchange_core_programs::parse_exchange;
#[path = "return_exchange/return_exchange_zone.rs"]
mod return_exchange_zone_programs;
pub use return_exchange_zone_programs::parse_return;
