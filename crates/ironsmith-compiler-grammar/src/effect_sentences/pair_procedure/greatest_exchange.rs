//! "You and target player exchange control of the creature you each control
//! with the greatest mana value. Then exchange control of artifacts the same
//! way. If two or more permanents a player controls are tied for greatest,
//! their controller chooses one of them." (Juxtapose): for each named card
//! type, each participant chooses one permanent of that type they control
//! whose mana value is the greatest among those permanents (the choice is the
//! tie-break, made by that permanent's controller), then the two chosen
//! permanents exchange control (CR 701.12). A participant with no such
//! permanent chooses nothing and that exchange does nothing.
use super::*;
use crate::cards::builders::{ExchangeActionAst, ObjectChoiceEffectAst};
use crate::lexer::OwnedLexToken;
use crate::types::CardType;
use crate::util::trim_edge_punctuation_tokens;

fn words(tokens: &[OwnedLexToken]) -> Vec<String> {
    crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(tokens))
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "land" | "lands" => Some(CardType::Land),
        _ => None,
    }
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(again), Some(tie)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let first_words = words(first.lowered());
    let first_words = first_words.iter().map(String::as_str).collect::<Vec<_>>();
    let (other, first_type) = match first_words.as_slice() {
        [
            "you", "and", "target", other @ ("player" | "opponent"), "exchange", "control", "of",
            "the", noun, "you", "each", "control", "with", "the", "greatest", "mana", "value",
        ] => (
            if *other == "player" { PlayerAst::Target } else { PlayerAst::TargetOpponent },
            card_type(noun),
        ),
        _ => return Ok(None),
    };
    let Some(first_type) = first_type else {
        return Ok(None);
    };
    let again_words = words(again.lowered());
    let again_words = again_words.iter().map(String::as_str).collect::<Vec<_>>();
    let again_words = again_words.strip_prefix(&["then"]).unwrap_or(&again_words);
    let Some(second_type) = (match again_words {
        ["exchange", "control", "of", noun, "the", "same", "way"] => card_type(noun),
        _ => None,
    }) else {
        return Ok(None);
    };
    let tie_words = words(tie.lowered());
    let tie_words = tie_words.iter().map(String::as_str).collect::<Vec<_>>();
    if !matches!(
        tie_words.as_slice(),
        [
            "if", "two", "or", "more", "permanents", "a", "player", "controls", "are", "tied",
            "for", "greatest", "their", "controller", "chooses", "one", "of", "them",
        ]
    ) {
        return Ok(None);
    }

    let greatest_of = |card_type: CardType, controller: PlayerFilter| {
        let mut scope = ObjectFilter::default().in_zone(Zone::Battlefield);
        scope.card_types = vec![card_type];
        scope.controller = Some(controller);
        let mut chosen = scope.clone();
        chosen.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestManaValue(scope),
        )));
        chosen
    };
    let mut effects = Vec::new();
    for (index, card_type) in [first_type, second_type].into_iter().enumerate() {
        let sentence = if index == 0 { first } else { again };
        let yours = helper_tag_for_tokens(sentence.lowered(), "greatest_exchange_yours");
        let theirs = helper_tag_for_tokens(sentence.lowered(), "greatest_exchange_theirs");
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: greatest_of(card_type, PlayerFilter::You),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: yours.clone(),
        }));
        // "you each control": the other participant's own permanents. The
        // participant is named directly by its target filter; an
        // IteratedPlayer outside any player loop lowers to "you", which made
        // the target player choose among the caster's permanents.
        let their_controller = match other {
            PlayerAst::TargetOpponent => PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
            _ => PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        };
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: greatest_of(card_type, their_controller),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: other,
            tag: theirs.clone(),
        }));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous {
                permanent1: TargetAst::Tagged(yours, None),
                permanent2: TargetAst::Tagged(theirs, None),
                shared_type: None,
            }),
        ));
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greatest_exchange_reads_both_card_types_with_owner_tie_choices() {
        let sentences = [
            "You and target player exchange control of the creature you each control with the greatest mana value",
            "Then exchange control of artifacts the same way",
            "If two or more permanents a player controls are tied for greatest, their controller chooses one of them",
        ]
        .map(|text| crate::lexer::lex_line(text, 0).unwrap());
        let inputs = sentences
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        let effects = read(&inputs, 0).unwrap().expect("greatest exchange");
        assert_eq!(effects.len(), 6, "{effects:?}");
        assert!(read(&inputs[..2], 0).unwrap().is_none());
    }
}
