//! "This turn, instead of declaring blockers, each defending player chooses
//! any number of creatures they control and divides them into a number of
//! piles equal to the number of attacking creatures for whom that player is
//! the defending player. Creatures those players control that can block
//! additional creatures may likewise be put into additional piles. Assign
//! each pile to a different one of those attacking creatures at random. Each
//! creature in a pile that can block the creature that pile is assigned to
//! does so." (Camouflage): the four statements are one replacement of this
//! turn's blocker declaration (CR 509.1), read together.
use super::*;

const STATEMENTS: [&[&str]; 4] = [
    &[
        "this", "turn", "instead", "of", "declaring", "blockers", "each", "defending", "player",
        "chooses", "any", "number", "of", "creatures", "they", "control", "and", "divides",
        "them", "into", "a", "number", "of", "piles", "equal", "to", "the", "number", "of",
        "attacking", "creatures", "for", "whom", "that", "player", "is", "the", "defending",
        "player",
    ],
    &[
        "creatures", "those", "players", "control", "that", "can", "block", "additional",
        "creatures", "may", "likewise", "be", "put", "into", "additional", "piles",
    ],
    &[
        "assign", "each", "pile", "to", "a", "different", "one", "of", "those", "attacking",
        "creatures", "at", "random",
    ],
    &[
        "each", "creature", "in", "a", "pile", "that", "can", "block", "the", "creature", "that",
        "pile", "is", "assigned", "to", "does", "so",
    ],
];

const REMINDER: &[&str] = &["piles", "can", "be", "empty"];

fn reads_statements(sentences: &[SentenceInput], index: usize) -> bool {
    STATEMENTS.iter().enumerate().all(|(offset, expected)| {
        sentences.get(index + offset).is_some_and(|sentence| {
            crate::lexer::parser_token_word_refs(sentence.lowered()).as_slice() == *expected
        })
    })
}

pub(crate) fn read(sentences: &[SentenceInput], index: usize) -> Option<Vec<EffectAst>> {
    reads_statements(sentences, index)
        .then(|| vec![EffectAst::DeclareBlockersByRandomPilesThisTurn])
}

/// The same statements followed by the "(Piles can be empty.)" reminder
/// when it reaches the sentence loop as its own sentence.
pub(crate) fn read_with_reminder(
    sentences: &[SentenceInput],
    index: usize,
) -> Option<Vec<EffectAst>> {
    let reminder = sentences.get(index + STATEMENTS.len())?;
    if crate::lexer::parser_token_word_refs(reminder.lowered()).as_slice() != REMINDER {
        return None;
    }
    read(sentences, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camouflage_statements_replace_this_turns_blocker_declaration() {
        let tokens = crate::lexer::lex_line(
            "This turn, instead of declaring blockers, each defending player chooses any number of creatures they control and divides them into a number of piles equal to the number of attacking creatures for whom that player is the defending player. Creatures those players control that can block additional creatures may likewise be put into additional piles. Assign each pile to a different one of those attacking creatures at random. Each creature in a pile that can block the creature that pile is assigned to does so.",
            0,
        )
        .unwrap();
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        assert_eq!(effects, vec![EffectAst::DeclareBlockersByRandomPilesThisTurn]);
    }
}
