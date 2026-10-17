//! "Choose a card in your hand. Defending player guesses whether that card's
//! mana value is greater than <N>. If they guessed wrong, you may cast it
//! without paying its mana cost. If you don't cast a spell this way,
//! investigate." (The Seventh Doctor). The guess is a choice between two
//! answers made by the defending player; whether it was wrong is checked
//! against the chosen card as the ability resolves (CR 608.2c). Read apart,
//! the guess and its follow-ups have no action of their own.
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, ConditionalEffectAst, EffectAst,
    ObjectChoiceEffectAst, OwnedLexToken, PermissionEffectAst, PlayerAst, PredicateAst,
};
use crate::effect::{ChoiceCount, Value};
use crate::lexer::{TokenWordView, parser_token_word_refs};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// The guessing player and the compared value. "Defending player" (an
/// attack trigger) or "that player" (the player a combat-damage trigger
/// damaged, Master of Predicaments).
fn compared_value(sentence: &[OwnedLexToken]) -> Option<(PlayerFilter, Value)> {
    let view = TokenWordView::new(sentence);
    let words = view.to_word_refs();
    let (guesser, rest) = if let Some(rest) = words.strip_prefix(&["defending", "player"][..]) {
        (PlayerFilter::Defending, rest)
    } else if let Some(rest) = words.strip_prefix(&["that", "player"][..]) {
        (PlayerFilter::DamagedPlayer, rest)
    } else {
        return None;
    };
    let rest = rest.strip_prefix(&["guesses", "whether"][..])?;
    let rest = rest
        .strip_prefix(&["that"][..])
        .or_else(|| rest.strip_prefix(&["the"][..]))?;
    let rest = rest
        .strip_prefix(&["card's"])
        .or_else(|| rest.strip_prefix(&["card", "s"]))
        .or_else(|| rest.strip_prefix(&["cards"]))?;
    let tail = rest.strip_prefix(&["mana", "value", "is", "greater", "than"])?;
    let start = words.len() - tail.len();
    let range = view.token_span_for_words(start, words.len())?;
    let value_tokens = crate::util::trim_edge_punctuation_tokens(&sentence[range]);
    let (value, used) = crate::util::parse_value(value_tokens)?;
    (used == value_tokens.len()).then_some((guesser, value))
}

/// "If they guessed wrong, you may cast it without paying its mana cost."
/// "If the player guessed wrong, you may cast the card without paying its
/// mana cost."
fn is_wrong_guess_free_cast(sentence: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(sentence);
    let Some(rest) = words
        .strip_prefix(&["if", "they"][..])
        .or_else(|| words.strip_prefix(&["if", "the", "player"][..]))
        .or_else(|| words.strip_prefix(&["if", "that", "player"][..]))
    else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(&["guessed", "wrong", "you", "may", "cast"][..]) else {
        return false;
    };
    matches!(
        rest,
        ["it", "without", "paying", "its", "mana", "cost"]
            | ["the" | "that", "card", "without", "paying", "its", "mana", "cost"]
    )
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (choose, guess, wrong, fallback) = match sentences {
        &[choose, guess, wrong, fallback] => (choose, guess, wrong, Some(fallback)),
        &[choose, guess, wrong] => (choose, guess, wrong, None),
        _ => return Ok(None),
    };
    if parser_token_word_refs(choose) != ["choose", "a", "card", "in", "your", "hand"]
        || !is_wrong_guess_free_cast(wrong)
        || fallback.is_some_and(|fallback| {
            !matches!(
                parser_token_word_refs(fallback).as_slice(),
                ["if", "you", "don't" | "dont", "cast", "a", "spell", "this", "way", "investigate"]
            )
        })
    {
        return Ok(None);
    }
    let Some((guesser, compared)) = compared_value(guess) else {
        return Ok(None);
    };
    let chosen = crate::util::helper_tag_for_tokens(choose, "guessed_card");
    let mut hand_card = ObjectFilter::default().in_zone(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::You);
    let cast = EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![EffectAst::subject_verb_cast_tagged(
            chosen.clone(),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        )],
    });
    // Without a fallback sentence a right guess does nothing.
    let investigate = fallback
        .map(|_| vec![EffectAst::subject_verb_investigate(PlayerAst::You, Value::Fixed(1))])
        .unwrap_or_default();
    // The fallback is bound to this exact optional cast (not to whatever
    // effect happens to precede it after lowering).
    let cast_or_investigate = if investigate.is_empty() {
        vec![cast]
    } else {
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen {
            effect: Box::new(cast),
            otherwise: investigate.clone(),
        })]
    };
    let greater = PredicateAst::ValueComparison {
        left: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(chosen.key.clone()))),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: compared,
    };
    let answer = |guessed_greater: bool| {
        let (if_true, if_false) = if guessed_greater {
            (investigate.clone(), cast_or_investigate.clone())
        } else {
            (cast_or_investigate.clone(), investigate.clone())
        };
        ChooseOneModeAst {
            description: if guessed_greater { "Greater" } else { "Not greater" }.to_string(),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: greater.clone(),
                if_true,
                if_false,
            })],
        }
    };
    Ok(Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: hand_card,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen,
        }),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
            chooser: guesser,
            modes: vec![answer(true), answer(false)],
        }),
    ]))
}
