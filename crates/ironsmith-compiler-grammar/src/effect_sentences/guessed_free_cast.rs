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

fn compared_value(sentence: &[OwnedLexToken]) -> Option<Value> {
    let view = TokenWordView::new(sentence);
    let words = view.to_word_refs();
    let head: &[&str] = &["defending", "player", "guesses", "whether", "that"];
    let rest = words.strip_prefix(head)?;
    let rest = rest
        .strip_prefix(&["card's"])
        .or_else(|| rest.strip_prefix(&["card", "s"]))
        .or_else(|| rest.strip_prefix(&["cards"]))?;
    let tail = rest.strip_prefix(&["mana", "value", "is", "greater", "than"])?;
    let start = words.len() - tail.len();
    let range = view.token_span_for_words(start, words.len())?;
    let value_tokens = crate::util::trim_edge_punctuation_tokens(&sentence[range]);
    let (value, used) = crate::util::parse_value(value_tokens)?;
    (used == value_tokens.len()).then_some(value)
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[choose, guess, wrong, fallback] = sentences else {
        return Ok(None);
    };
    if parser_token_word_refs(choose) != ["choose", "a", "card", "in", "your", "hand"]
        || parser_token_word_refs(wrong)
            != [
                "if", "they", "guessed", "wrong", "you", "may", "cast", "it", "without", "paying",
                "its", "mana", "cost",
            ]
        || !matches!(
            parser_token_word_refs(fallback).as_slice(),
            ["if", "you", "don't" | "dont", "cast", "a", "spell", "this", "way", "investigate"]
        )
    {
        return Ok(None);
    }
    let Some(compared) = compared_value(guess) else {
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
    let investigate = EffectAst::subject_verb_investigate(PlayerAst::You, Value::Fixed(1));
    // The fallback is bound to this exact optional cast (not to whatever
    // effect happens to precede it after lowering).
    let cast_or_investigate = vec![EffectAst::Conditionals(
        ConditionalEffectAst::IfEffectDidNotHappen {
            effect: Box::new(cast),
            otherwise: vec![investigate.clone()],
        },
    )];
    let greater = PredicateAst::ValueComparison {
        left: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(chosen.key.clone()))),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: compared,
    };
    let answer = |guessed_greater: bool| {
        let (if_true, if_false) = if guessed_greater {
            (vec![investigate.clone()], cast_or_investigate.clone())
        } else {
            (cast_or_investigate.clone(), vec![investigate.clone()])
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
            chooser: PlayerFilter::Defending,
            modes: vec![answer(true), answer(false)],
        }),
    ]))
}
