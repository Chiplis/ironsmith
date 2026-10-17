//! "..., then choose land or nonland. An opponent guesses whether the top
//! card of your library is the chosen kind. Reveal that card. If they guessed
//! right, <right>. Otherwise, <wrong>." (Gollum, Scheming Guide): you choose
//! a kind, then an opponent makes a two-answer guess; whether it was right is
//! checked against the top card of your library as the ability resolves
//! (CR 608.2c). The kind choice and the guess are both public choices, so
//! they are modeled as nested modal choices whose leaves reveal the card and
//! branch on its kind.
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, ConditionalEffectAst, EffectAst, ObjectChoiceEffectAst,
    OwnedLexToken, PredicateAst,
};
use crate::lexer::parser_token_word_refs;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

const CHOOSE_KIND_SUFFIX: &[&str] = &["then", "choose", "land", "or", "nonland"];

/// The sentence before the guess with its trailing ", then choose land or
/// nonland" removed.
fn strip_kind_choice(sentence: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    if !words.ends_with(CHOOSE_KIND_SUFFIX) {
        return None;
    }
    // Walk back over the five suffix words, then any separating commas.
    let mut end = sentence.len();
    let mut remaining = CHOOSE_KIND_SUFFIX.len();
    while remaining > 0 {
        end = end.checked_sub(1)?;
        if sentence[end].as_word().is_some() {
            remaining -= 1;
        }
    }
    while end > 0 && sentence[end - 1].as_word().is_none() {
        end -= 1;
    }
    (end > 0).then(|| &sentence[..end])
}

/// "If they guessed right, <effects>" / "Otherwise, <effects>": the tail.
fn strip_lead<'a>(sentence: &'a [OwnedLexToken], lead: &[&str]) -> Option<&'a [OwnedLexToken]> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let mut index = 0usize;
    for expected in lead {
        while index < sentence.len() && sentence[index].as_word().is_none() {
            index += 1;
        }
        let token = sentence.get(index)?;
        if !token.is_word(expected) {
            return None;
        }
        index += 1;
    }
    let tail = crate::util::trim_edge_punctuation_tokens(&sentence[index..]);
    (!tail.is_empty()).then_some(tail)
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[setup, guess, reveal, right, wrong] = sentences else {
        return Ok(None);
    };
    let Some(setup) = strip_kind_choice(setup) else {
        return Ok(None);
    };
    if !matches!(
        parser_token_word_refs(guess).as_slice(),
        [
            "an", "opponent", "guesses", "whether", "the", "top", "card", "of", "your",
            "library", "is", "the", "chosen", "kind"
        ]
    ) || !matches!(
        parser_token_word_refs(reveal).as_slice(),
        ["reveal", "that", "card"] | ["reveal", "the", "top", "card", "of", "your", "library"]
    ) {
        return Ok(None);
    }
    let (Some(right), Some(wrong)) = (
        strip_lead(right, &["if", "they", "guessed", "right"]),
        strip_lead(wrong, &["otherwise"]),
    ) else {
        return Ok(None);
    };
    let setup_effects = super::parse_effect_chain(setup)?;
    let right_effects = super::parse_effect_chain(right)?;
    let wrong_effects = super::parse_effect_chain(wrong)?;
    let reveal_tokens = crate::lexer::lex_line("reveal the top card of your library", 0)?;
    let reveal_effects = super::parse_effect_chain(&reveal_tokens)?;

    let kind_mode = |label: &str, filter: ObjectFilter| {
        let filter = filter.in_zone(Zone::Library);
        let guess_mode = |guessed_yes: bool| {
            let (if_true, if_false) = if guessed_yes {
                (right_effects.clone(), wrong_effects.clone())
            } else {
                (wrong_effects.clone(), right_effects.clone())
            };
            let mut effects = reveal_effects.clone();
            effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::TopCardOfYourLibraryMatches(filter.clone()),
                if_true,
                if_false,
            }));
            ChooseOneModeAst {
                description: if guessed_yes {
                    format!("It's {label}")
                } else {
                    format!("It isn't {label}")
                },
                effects,
            }
        };
        ChooseOneModeAst {
            description: label.to_string(),
            effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
                chooser: PlayerFilter::Opponent,
                modes: vec![guess_mode(true), guess_mode(false)],
            })],
        }
    };
    let mut land = ObjectFilter::default();
    land.card_types.push(CardType::Land);
    let mut nonland = ObjectFilter::default();
    nonland.excluded_card_types.push(CardType::Land);

    let mut effects = setup_effects;
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
        chooser: PlayerFilter::You,
        modes: vec![kind_mode("Land", land), kind_mode("Nonland", nonland)],
    }));
    Ok(Some(effects))
}
