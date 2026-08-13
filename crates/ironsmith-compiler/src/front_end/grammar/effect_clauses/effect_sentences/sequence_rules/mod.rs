use super::super::token_primitives::lexed_head_words;
use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::LegacyOrderRank;

pub(super) mod generic_subject_verb_sequences;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{IfResultPredicate, SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::runtime_backend::{lex_line, split_lexed_sentences};

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
            first_word_then_target_exile_look_or_reveal(&sentences, 0),
            "leading-then predicate must admit the sentence"
        );
        assert!(
            generic_subject_verb_sequences::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom(
                &sentences,
                0,
            )
            .expect("specialized parser")
            .is_some(),
            "specialized looked-partition parser must accept the three-sentence shape"
        );
        let matched = try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("sequence parse")
            .expect("leading-then looked partition should match a typed sequence rule");
        assert_eq!(
            matched.name,
            "top-cards-put-any-matching-to-zone-rest-bottom"
        );

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

        let matched = try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("sequence parse")
            .expect("the optional participant process should match");
        assert_eq!(matched.name, "starting-each-player-optional-repeat");
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
