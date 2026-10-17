//! "Each player may draw up to two cards. For each card less than two a
//! player draws this way, that player gains 2 life." (Truce): each player
//! independently decides card by card; every card not drawn yields the gain.
//! The program is that many rounds of "each player may draw a card; each
//! player who doesn't gains N life", which gives exactly the authored totals.
use super::*;
use crate::cards::builders::{ForEachEffectAst, LifeResourceActionAst, PermissionEffectAst};
use crate::lexer::OwnedLexToken;
use crate::util::trim_edge_punctuation_tokens;

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
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: Value::Fixed(1) }),
    );
    let gain = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::That,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
            amount: Value::Fixed(amount as i32),
        }),
    );
    let mut effects = Vec::new();
    for _ in 0..limit {
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![draw.clone()],
            })],
        }));
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDoesNot {
            effects: vec![gain.clone()],
            predicate: None,
        }));
    }
    Ok(Some(effects))
}
