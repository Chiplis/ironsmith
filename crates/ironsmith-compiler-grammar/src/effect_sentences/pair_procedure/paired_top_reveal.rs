//! "You and target opponent each reveal the top card of your library. You
//! each lose life equal to the mana value of the card revealed by the other
//! player. You each put the card you revealed into your hand." (Keen
//! Duelist): a two-participant reveal whose follow-up statements cross-refer
//! each participant's revealed card. Each reveal is tagged separately so the
//! life loss reads the *other* participant's card and the final move keeps
//! each participant's own card.
use super::*;
use crate::cards::builders::LifeResourceActionAst;
use crate::lexer::OwnedLexToken;
use crate::util::trim_edge_punctuation_tokens;

fn normalized(tokens: &[OwnedLexToken]) -> String {
    let words = crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(tokens));
    let words = words.strip_prefix(&["then"]).unwrap_or(&words);
    words.join(" ").replace(['\'', '’'], "")
}

fn other_participant(reveal: &str) -> Option<PlayerAst> {
    let rest = reveal.strip_prefix("you and ")?;
    let (player, rest) = if let Some(rest) = rest.strip_prefix("target opponent ") {
        (PlayerAst::TargetOpponent, rest)
    } else if let Some(rest) = rest.strip_prefix("target player ") {
        (PlayerAst::Target, rest)
    } else if let Some(rest) = rest.strip_prefix("that player ") {
        (PlayerAst::That, rest)
    } else {
        return None;
    };
    matches!(
        rest,
        "each reveal the top card of your library"
            | "each reveal the top card of your libraries"
            | "each reveal the top card of their library"
    )
    .then_some(player)
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(reveal), Some(loss), Some(keep)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let Some(other) = other_participant(&normalized(reveal.lowered())) else {
        return Ok(None);
    };
    if normalized(loss.lowered())
        != "you each lose life equal to the mana value of the card revealed by the other player"
        || !matches!(
            normalized(keep.lowered()).as_str(),
            "you each put the card you revealed into your hand"
                | "you each put the card you revealed into your hands"
        )
    {
        return Ok(None);
    }
    let yours = helper_tag_for_tokens(reveal.lowered(), "paired_reveal_yours");
    let theirs = helper_tag_for_tokens(reveal.lowered(), "paired_reveal_theirs");
    let mana_value_of = |tag: &crate::tag::TagRef| {
        Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(tag.key.clone())))
    };
    let lose = |player: PlayerAst, amount: Value| {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
        )
    };
    let to_hand = |tag: &crate::tag::TagRef| {
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(tag.clone(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
    };
    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(PlayerAst::You, Value::Fixed(1), yours.clone()),
        EffectAst::subject_verb_reveal_top_cards(other, Value::Fixed(1), theirs.clone()),
        lose(PlayerAst::You, mana_value_of(&theirs)),
        lose(other, mana_value_of(&yours)),
        to_hand(&yours),
        to_hand(&theirs),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_top_reveal_cross_refers_each_participants_card() {
        let sentences = [
            "You and target opponent each reveal the top card of your library",
            "You each lose life equal to the mana value of the card revealed by the other player",
            "You each put the card you revealed into your hand",
        ]
        .map(|text| crate::lexer::lex_line(text, 0).unwrap());
        let inputs = sentences
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        let effects = read(&inputs, 0).unwrap().expect("paired top reveal");
        assert_eq!(effects.len(), 6, "{effects:?}");
        assert!(read(&inputs[..2], 0).unwrap().is_none());
    }
}
