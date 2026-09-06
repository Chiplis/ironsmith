use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::LibraryActionAst;
use super::*;
use crate::{lex_line, split_lexed_sentences};

#[test]
fn global_destroy_keeps_targeted_search_owner_chooser_and_destination_separate() {
    let tokens = lex_line(
            "Destroy all creatures, then search target opponent's library for up to three creature cards and put them into their graveyard. Then that player shuffles.",
            0,
        )
        .expect("destroy/search probe should lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects =
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .expect("destroy/search parser should not error")
            .expect("destroy/search shape should match");

    let [destroy, target, search, shuffle] = effects.as_slice() else {
        panic!("expected destroy/target/search/shuffle effects: {effects:#?}");
    };
    assert!(matches!(
        destroy,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { .. }),
            ..
        })
    ));
    assert!(matches!(
        target,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Player(player, _),
                ..
            },
            ..
        }) if target_opponent_filter(player)
    ));
    assert!(matches!(
        search,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                filter,
                destination: Zone::Graveyard,
                chooser: PlayerAst::Implicit,
                player: PlayerAst::That,
                count,
                shuffle: false,
                ..
            }),
            ..
        }) if filter.zone == Some(Zone::Library)
            && filter.owner.as_ref().is_some_and(target_opponent_filter)
            && filter.card_types == vec![CardType::Creature]
            && count == &ChoiceCount::up_to(3)
    ));
    assert!(matches!(
        shuffle,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::LibraryOwner,
                player: PlayerAst::That,
                ..
            },
            action: SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        })
    ));
}
