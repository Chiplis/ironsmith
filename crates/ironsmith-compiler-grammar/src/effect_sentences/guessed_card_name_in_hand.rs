//! "Choose a card name. Target opponent guesses whether a card with that name
//! is in your hand. You may reveal your hand. If you do and your opponent
//! guessed wrong, draw a card." (Liar's Pendulum): you name a card, then a
//! player makes a two-answer guess. The guess is a public choice between the
//! two answers; whether it was wrong is checked against your hand as the
//! ability resolves (CR 608.2c), and only matters when you accept the
//! optional reveal. Read apart, the guess and its follow-up have no action
//! of their own.
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, ConditionalEffectAst, EffectAst, IfResultPredicate,
    ObjectChoiceEffectAst, OwnedLexToken, PredicateAst, TargetAst,
};
use crate::effect::{Value, ValueComparisonOperator};
use crate::lexer::parser_token_word_refs;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::zone::Zone;

/// The guessing player, and the target it declares when it is a target.
fn guesser(sentence: &[OwnedLexToken]) -> Option<(PlayerFilter, Option<TargetAst>)> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    let (guesser, target, rest) = match words.as_slice() {
        ["target", "opponent", rest @ ..] => (
            PlayerFilter::target_opponent(),
            Some(TargetAst::Player(
                PlayerFilter::Opponent,
                crate::util::span_from_tokens(&sentence[..2]),
            )),
            rest,
        ),
        ["target", "player", rest @ ..] => (
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
            Some(TargetAst::Player(
                PlayerFilter::Any,
                crate::util::span_from_tokens(&sentence[..2]),
            )),
            rest,
        ),
        ["an", "opponent", rest @ ..] => (PlayerFilter::Opponent, None, rest),
        _ => return None,
    };
    let tail = match rest {
        ["guesses", "whether", "a", "card", "with", "that", "name", tail @ ..] => tail,
        ["guesses", "whether", "a", "card", "with", "the", "chosen", "name", tail @ ..] => tail,
        _ => return None,
    };
    matches!(tail, ["is", "in", "your", "hand"]).then_some((guesser, target))
}

/// "If you do and your opponent guessed wrong, <effects>": the tail.
fn wrong_guess_tail(sentence: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    let lead: &[&str] = match words.as_slice() {
        ["if", "you", "do", "and", "your", "opponent", "guessed", "wrong", ..] => {
            &["if", "you", "do", "and", "your", "opponent", "guessed", "wrong"]
        }
        ["if", "you", "do", "and", "that", "player" | "opponent", "guessed", "wrong", ..] => {
            &["if", "you", "do", "and", "that", "player", "guessed", "wrong"]
        }
        ["if", "you", "do", "and", "they", "guessed", "wrong", ..] => {
            &["if", "you", "do", "and", "they", "guessed", "wrong"]
        }
        _ => return None,
    };
    // Walk past the lead's words (and any punctuation between them).
    let mut index = 0usize;
    let mut remaining = lead.len();
    while remaining > 0 {
        let token = sentence.get(index)?;
        if token.as_word().is_some() {
            remaining -= 1;
        }
        index += 1;
    }
    let tail = crate::util::trim_edge_punctuation_tokens(&sentence[index..]);
    (!tail.is_empty()).then_some(tail)
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[choose, guess, reveal, wrong] = sentences else {
        return Ok(None);
    };
    if parser_token_word_refs(crate::util::trim_edge_punctuation_tokens(choose))
        != ["choose", "a", "card", "name"]
        || parser_token_word_refs(crate::util::trim_edge_punctuation_tokens(reveal))
            != ["you", "may", "reveal", "your", "hand"]
    {
        return Ok(None);
    }
    let Some((guesser, target)) = guesser(guess) else {
        return Ok(None);
    };
    let Some(wrong) = wrong_guess_tail(wrong) else {
        return Ok(None);
    };
    let setup_effects = super::parse_effect_chain(choose)?;
    let reveal_effects = super::parse_effect_chain(reveal)?;
    let wrong_effects = super::parse_effect_chain(wrong)?;

    // "a card with that name is in your hand", read with the chosen name in
    // scope as the guess resolves.
    let named_in_hand = PredicateAst::ValueComparison {
        left: Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You)
                .match_tagged(
                    crate::tag::CompilerReferenceTag::ChosenName.bind(),
                    TaggedOpbjectRelation::SameNameAsTagged,
                ),
        ),
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    };
    let answer = |guessed_in_hand: bool| {
        let wrong_when = if guessed_in_hand {
            PredicateAst::Not(Box::new(named_in_hand.clone()))
        } else {
            named_in_hand.clone()
        };
        let mut effects = reveal_effects.clone();
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: wrong_when,
                if_true: wrong_effects.clone(),
                if_false: Vec::new(),
            })],
        }));
        ChooseOneModeAst {
            description: if guessed_in_hand {
                "It's in their hand"
            } else {
                "It isn't in their hand"
            }
            .to_string(),
            effects,
        }
    };

    let mut effects = setup_effects;
    if let Some(target) = target {
        effects.push(EffectAst::subject_verb_target_only(target));
    }
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
        chooser: guesser,
        modes: vec![answer(true), answer(false)],
    }));
    Ok(Some(effects))
}
