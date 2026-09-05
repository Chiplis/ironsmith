use super::super::token_primitives::lexed_head_words;
use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};

pub(super) mod generic_subject_verb_sequences;

#[derive(Debug, Clone)]
pub struct DocumentProgramMatch {
    pub name: &'static str,
    pub feature_tag: Option<&'static str>,
    pub consumed_sentences: usize,
    pub effects: Vec<EffectAst>,
}

fn sentence_head(sentences: &[SentenceInput], sentence_idx: usize) -> Option<(&str, Option<&str>)> {
    lexed_head_words(sentences[sentence_idx].lowered())
}

pub(crate) fn sentence_head_word(sentences: &[SentenceInput], sentence_idx: usize) -> Option<&str> {
    sentence_head(sentences, sentence_idx).map(|(head, _)| head)
}

pub(crate) fn sentence_head_is(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    expected: (&str, Option<&str>),
) -> bool {
    sentence_head(sentences, sentence_idx) == Some(expected)
}

pub(crate) fn sentence_head_word_is(sentences: &[SentenceInput], sentence_idx: usize, expected: &str) -> bool {
    sentence_head_word(sentences, sentence_idx) == Some(expected)
}

pub(crate) fn sentence_head_word_in(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    expected: &[&str],
) -> bool {
    sentence_head_word(sentences, sentence_idx).is_some_and(|head| expected.contains(&head))
}

pub(crate) fn sentence_words_contain(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    phrase: &[&str],
) -> bool {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return false;
    };
    let words = crate::lexer::token_word_refs(sentence.lowered());
    crate::word_primitives::sequence_occurs(&words, phrase)
}

/// Read the multi-sentence procedure that opens at this sentence, if one does.
pub fn try_parse_document_program(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<DocumentProgramMatch>, CardTextError> {
    super::procedures::recognize(sentences, sentence_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{IfResultPredicate, SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::{lex_line, split_lexed_sentences};

    #[test]
    fn leading_then_looked_partition_uses_one_provenance_program() {
        let tokens = lex_line(
            "Then look at the top X cards of your library, where X is the number of time counters on this creature. You may put a nonland permanent card with mana value 3 or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        assert!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
                &sentences,
                0,
            )
            .expect("specialized parser")
            .is_some(),
            "specialized looked-partition parser must accept the three-sentence shape"
        );
        let matched = DocumentProgramMatch {
            name: "looked-procedure",
            feature_tag: None,
            consumed_sentences: sentences.len(),
            effects: crate::clause_support::parse_effect_sentences_lexed(&tokens)
                .expect("leading-then looked partition should parse as composed statements"),
        };

        let [look, choose, move_each, remainder] = matched.effects.as_slice() else {
            panic!(
                "expected look/choose/move/remainder provenance program: {:#?}",
                matched.effects
            );
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { tag: looked, .. },
            ..
        }) = look
        else {
            panic!("expected looked-card producer: {look:#?}");
        };
        let EffectAst::ChooseTaggedObjectsInZone {
            filter,
            tag: chosen,
            ..
        } = choose
        else {
            panic!("expected looked-card selection: {choose:#?}");
        };
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag == *looked),
            "selection must consume the looked-card pool: {filter:#?}"
        );
        assert!(matches!(
            move_each,
            EffectAst::ForEachTagged { tag, .. } if tag == chosen
        ));
        assert!(matches!(
            remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        ..
                    },
                ..
            }) if tag == looked && keep_tagged == chosen
        ));
    }

    #[test]
    fn conditional_looked_partition_keeps_the_full_looked_collection_for_the_remainder() {
        let tokens = lex_line(
            "If you do, look at the top X cards of your library, where X is that creature's mana value. You may put a creature card from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        let matched = DocumentProgramMatch {
            name: "looked-procedure",
            feature_tag: None,
            consumed_sentences: sentences.len(),
            effects: crate::clause_support::parse_effect_sentences_lexed(&tokens)
                .expect("conditional looked partition should parse as composed statements"),
        };

        let [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects,
            },
        ] = matched.effects.as_slice()
        else {
            panic!(
                "expected one conditional looked partition: {:#?}",
                matched.effects
            );
        };
        let [look, choose, move_each, remainder] = effects.as_slice() else {
            panic!("expected look/choose/move/remainder effects: {effects:#?}");
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { tag: looked, .. },
            ..
        }) = look
        else {
            panic!("expected a looked-card producer: {look:#?}");
        };
        let EffectAst::ChooseTaggedObjectsInZone { tag: chosen, .. } = choose else {
            panic!("expected a typed looked-card choice: {choose:#?}");
        };
        assert!(matches!(move_each, EffectAst::ForEachTagged { tag, .. } if tag == chosen));
        assert!(matches!(
            remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: crate::cards::builders::LibraryBottomOrderAst::Random,
                    ..
                },
                ..
            }) if tag == looked && keep_tagged == chosen
        ));
    }

    #[test]
    fn starting_each_player_optional_action_becomes_one_typed_repeat_process() {
        let tokens = lex_line(
            "Starting with you, each player may put a permanent card from their hand onto the battlefield. Repeat this process until no one puts a card onto the battlefield.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        let matched = try_parse_document_program(&sentences, 0)
            .expect("sequence parse")
            .expect("the optional participant process should match");
        assert_eq!(matched.consumed_sentences, 2);

        let [
            EffectAst::RepeatProcess {
                effects,
                continue_effect_index,
                continue_predicate: IfResultPredicate::Did,
            },
        ] = matched.effects.as_slice()
        else {
            panic!(
                "expected one typed repeat process, got: {:#?}",
                matched.effects
            );
        };
        assert_eq!(*continue_effect_index, 0);
        let [
            EffectAst::SourceSentence {
                effects,
                starting_with_controller: true,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("the repeat body must retain authored participant order: {effects:#?}");
        };
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::ForEachPlayer {
                effects: per_player,
            }] if matches!(per_player.as_slice(), [EffectAst::May { .. }])
        ));
    }
}
