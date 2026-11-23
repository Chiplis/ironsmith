//! "For as long as they have a contract counter, when they lose the game,
//! for each artifact and creature they controlled, create a token that's a
//! copy of it." (Miss Highwater)
//!
//! A delayed ability registered for the player the resolving ability names
//! ("they"), watching that player lose the game (CR 104.3). Its lifetime is
//! bounded by the player keeping a counter of the named kind: the counter is
//! checked as the player loses, and the registration is made only while the
//! player has one. The permanents the player controlled leave the game with
//! them (CR 800.4a); the delayed ability iterates their last known
//! information, so "it" is each such permanent as it last existed.
use super::*;
use crate::cards::builders::{
    ConditionalEffectAst, DelayedEffectAst, ForEachEffectAst, TriggerSpec,
};
use crate::types::CardType;

fn card_type_word(word: &str) -> Option<CardType> {
    match word {
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "creature" | "creatures" => Some(CardType::Creature),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "battle" | "battles" => Some(CardType::Battle),
        _ => None,
    }
}

pub(crate) fn parse_tokens(
    tokens: &[crate::lexer::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = crate::lexer::parser_token_word_refs(tokens);
    if !words_all.starts_with(&["for", "as", "long", "as"]) && !words_all.starts_with(&["when"]) {
        return Ok(None);
    }
    let tokens = trim_commas(tokens);
    // Split at commas: [duration,] trigger, iteration, instruction.
    let mut parts: Vec<&[crate::lexer::OwnedLexToken]> = Vec::new();
    let mut start = 0;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_comma() {
            parts.push(&tokens[start..idx]);
            start = idx + 1;
        }
    }
    parts.push(&tokens[start..]);
    fn words(part: &[crate::lexer::OwnedLexToken]) -> Vec<&str> { crate::lexer::parser_token_word_refs(part) }
    let (counter, rest) = match parts.as_slice() {
        [duration, rest @ ..] if words(duration).starts_with(&["for", "as", "long", "as"]) => {
            let duration_words = words(duration);
            let counter_words = match duration_words.as_slice() {
                ["for", "as", "long", "as", "they" | "that", rest @ ..] => {
                    let rest = rest.strip_prefix(&["player"]).unwrap_or(rest);
                    match rest {
                        ["have" | "has", "a" | "an", counter @ .., "counter"] if !counter.is_empty() => {
                            let mut kind = counter.to_vec();
                            kind.push("counter");
                            kind
                        }
                        _ => return Ok(None),
                    }
                }
                _ => return Ok(None),
            };
            let Some(counter) = crate::grammar::filters::parse_counter_type_words(&counter_words)
            else {
                return Ok(None);
            };
            (Some((counter, counter_words.join(" "))), rest)
        }
        rest => (None, rest),
    };
    let [trigger, iteration, instruction] = rest else {
        return Ok(None);
    };
    if !matches!(
        words(trigger).as_slice(),
        ["when", "they", "lose", "the", "game"] | ["when", "that", "player", "loses", "the", "game"]
    ) {
        return Ok(None);
    }
    let iteration_words = words(iteration);
    let ["for", "each", types @ .., "they" | "that", "controlled"] = iteration_words.as_slice()
    else {
        return Ok(None);
    };
    let types = types.strip_suffix(&["player"]).unwrap_or(types);
    let mut card_types = Vec::new();
    for word in types {
        match *word {
            "and" | "or" | "and/or" => {}
            word => card_types.push(card_type_word(word).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported lost-player permanent type '{word}'"
                ))
            })?),
        }
    }
    if card_types.is_empty() {
        return Ok(None);
    }
    let body = super::super::parse_effect_sentence_lexed(instruction)?;
    if body.is_empty() {
        return Ok(None);
    }
    let mut filter = ObjectFilter::default();
    filter.card_types = card_types;
    let lost = crate::tag::TagRef::of(ironsmith_core::tag::LOST_PLAYER_CONTROLLED_PERMANENTS_TAG);
    let per_permanent = EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: lost,
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatchedLastKnown(
                crate::tag::CompilerReferenceTag::It.bind(),
                filter,
            ),
            if_true: body,
            if_false: Vec::new(),
        })],
    });
    let loses = TriggerSpec::WithIntro {
        intro: crate::model::ast::TriggerIntroSurfaceAst::When,
        trigger: Box::new(TriggerSpec::PlayerLosesGame(PlayerFilter::IteratedPlayer)),
    };
    let has_counter = |counter: &crate::object::CounterType| PredicateAst::ValueComparison {
        left: Value::PlayerCounters(PlayerFilter::IteratedPlayer, counter.clone()),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    };
    let trigger = match &counter {
        Some((counter, phrase)) => TriggerSpec::ConditionQualified {
            trigger: Box::new(loses),
            condition: has_counter(counter),
            surface: format!("for as long as they have a {phrase}"),
        },
        None => loses,
    };
    // The duration, trigger and iteration parts were read word for word;
    // the instruction is its own sentence reading.
    for part in &parts[..parts.len() - 1] {
        crate::word_claims::claim_tokens(part);
    }
    let delayed = EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
        trigger,
        effects: vec![per_permanent],
        one_shot: true,
        duration: crate::effect::Until::Forever,
        either_of_watched_objects: false,
        while_any_tagged_object_in_zone: None,
        source_collection_scope: None,
        source_absence_state: false,
    });
    Ok(Some(vec![match &counter {
        // The registration exists only while the player has the counter.
        Some((counter, _)) => EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: has_counter(counter),
            if_true: vec![delayed],
            if_false: Vec::new(),
        }),
        None => delayed,
    }]))
}
