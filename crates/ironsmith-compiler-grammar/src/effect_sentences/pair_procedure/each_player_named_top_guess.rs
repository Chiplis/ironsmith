//! "Then each player reveals the top card of their library. If the card a
//! player revealed has the name they chose, that player puts it into their
//! hand. If it doesn't, that player puts it on the bottom of their library."
//! (Conundrum Sphinx): after every player has chosen a card name, each
//! player's own revealed card is compared with the name *that player* chose.
//! The comparison reads the accumulated chosen-name tag restricted to the
//! revealed card's owner, so no player's reveal precedes another's choice and
//! no player is judged by another player's guess.
use super::*;
use crate::cards::builders::{ConditionalEffectAst, ForEachEffectAst};
use crate::lexer::OwnedLexToken;
use crate::util::trim_edge_punctuation_tokens;

fn normalized(tokens: &[OwnedLexToken]) -> String {
    let words = crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(tokens));
    let words = words.strip_prefix(&["then"]).unwrap_or(&words);
    words
        .join(" ")
        .replace(['\'', '’'], "")
        .replace("doesn t", "doesnt")
        .replace("does not", "doesnt")
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(reveal), Some(matched), Some(unmatched)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    if normalized(reveal.lowered()) != "each player reveals the top card of their library"
        || normalized(matched.lowered())
            != "if the card a player revealed has the name they chose that player puts it into their hand"
        || normalized(unmatched.lowered())
            != "if it doesnt that player puts it on the bottom of their library"
    {
        return Ok(None);
    }
    let it = crate::tag::CompilerReferenceTag::It.bind();
    let guessed = ObjectFilter::default().match_tagged(
        crate::tag::CompilerReferenceTag::ChosenName.bind(),
        crate::target::TaggedOpbjectRelation::SameNameAsTaggedChosenByOwner,
    );
    let to_hand = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(it.clone(), None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    );
    let to_bottom = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(it.clone(), None),
        Zone::Library,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    );
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_reveal_top(PlayerAst::That),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(it, guessed),
                if_true: vec![to_hand],
                if_false: vec![to_bottom],
            }),
        ],
    })]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_player_named_top_guess_reads_three_sentences_as_one_player_loop() {
        let sentences = [
            "Then each player reveals the top card of their library",
            "If the card a player revealed has the name they chose, that player puts it into their hand",
            "If it doesn't, that player puts it on the bottom of their library",
        ]
        .map(|text| crate::lexer::lex_line(text, 0).unwrap());
        let inputs = sentences
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        let effects = read(&inputs, 0).unwrap().expect("named top guess");
        let [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })] = effects.as_slice()
        else {
            panic!("one per-player loop: {effects:?}");
        };
        assert_eq!(effects.len(), 2);
        assert!(read(&inputs[..2], 0).unwrap().is_none());
    }
}
