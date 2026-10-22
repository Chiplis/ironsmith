//! "Create <token>. Then for each opponent with three or more poison
//! counters, you create another one of those tokens." (Wurmquake): the second
//! sentence repeats the first sentence's token creation once per qualifying
//! opponent, with you as the creator (not the iterated opponent).
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst, OwnedLexToken, PlayerAst,
    PlayerPredicateAst, PredicateAst,
};
use crate::lexer::token_word_refs;

/// The poison-counter threshold of "for each opponent with N or more poison
/// counters, you create another one of those tokens".
fn poison_threshold(sentence: &[OwnedLexToken]) -> Option<u32> {
    let words = token_word_refs(sentence);
    let words = words.strip_prefix(&["then"][..]).unwrap_or(&words);
    let [
        "for",
        "each",
        "opponent",
        "with",
        count,
        "or",
        "more",
        "poison",
        "counters",
        "you",
        "create",
        "another",
        "one",
        "of",
        "those",
        "tokens",
    ] = words
    else {
        return None;
    };
    crate::util::parse_number_word_u32(count).or_else(|| count.parse::<u32>().ok())
}

pub(crate) fn read(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[create, repeat] = sentences else {
        return Ok(None);
    };
    if !create.first().is_some_and(|token| token.is_word("create")) {
        return Ok(None);
    }
    let Some(count) = poison_threshold(repeat) else {
        return Ok(None);
    };
    let first = super::parse_effect_sentence_lexed(create)?;
    let [created] = first.as_slice() else {
        return Ok(None);
    };
    let mut again = created.clone();
    super::chain_carry::bind_implicit_player_context(&mut again, PlayerAst::You);
    let per_opponent = EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::Player(PlayerPredicateAst::PlayerHasPoisonCountersOrMore {
                player: PlayerAst::That,
                count,
                as_you_cast: false,
            }),
            if_true: vec![again],
            if_false: Vec::new(),
        })],
    });
    Ok(Some(vec![created.clone(), per_opponent]))
}
