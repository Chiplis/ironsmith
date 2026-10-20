//! "Each player may draw up to two cards. For each card less than two a
//! player draws this way, that player gains 2 life." Each player chooses
//! their draw count before anyone draws. The draw instruction then resolves
//! for all players, and only afterward does the life gain read each player's
//! actual draw receipt. Separate passes preserve draw order and replacements.
use super::*;
use crate::cards::builders::{ForEachEffectAst, LifeResourceActionAst};
use crate::lexer::OwnedLexToken;
use crate::util::trim_edge_punctuation_tokens;
use ironsmith_core::{
    ChoosePlayerOptionEffect, EffectMetric, EffectMetricSource, PlayerNumberChoice,
    PriorEffectAction, PriorEffectMetricQuery,
};

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(tokens))
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_words = words(first.lowered());
    let second_words = words(second.lowered());
    let ["each", "player", "may", "draw", "up", "to", limit, "cards"] = first_words.as_slice()
    else {
        return Ok(None);
    };
    let [
        "for", "each", "card", "less", "than", shortfall_limit, "a", "player", "draws", "this",
        "way", "that", "player", "gains", amount, "life",
    ] = second_words.as_slice()
    else {
        return Ok(None);
    };
    let (Some(limit), Some(shortfall_limit), Some(amount)) = (
        crate::util::parse_number_word_u32(limit),
        crate::util::parse_number_word_u32(shortfall_limit),
        crate::util::parse_number_word_u32(amount),
    ) else {
        return Ok(None);
    };
    if limit != shortfall_limit || limit == 0 || limit > 10 {
        return Ok(None);
    }
    let draw = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::That,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: Value::IteratedPlayerChosenNumber,
        }),
    );
    let actual_draws = Value::PendingPriorEffectMetric(
        PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_action(PriorEffectAction::Drawn)
            .with_player(PlayerFilter::IteratedPlayer),
    );
    let shortfall = Value::Add(
        Box::new(Value::Fixed(limit as i32)),
        Box::new(Value::Scaled(Box::new(actual_draws), -1)),
    );
    let gain = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::That,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
            amount: Value::Scaled(Box::new(shortfall), amount as i32),
        }),
    );
    Ok(Some(vec![
        EffectAst::ChoosePlayerOption(
            ChoosePlayerOptionEffect::numbers(
                PlayerFilter::Any,
                PlayerNumberChoice {
                    min: 0,
                    max: Some(limit),
                },
            )
            .for_draw_count(),
        ),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![draw],
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![gain],
        }),
    ]))
}
