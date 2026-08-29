use super::*;

fn search_subjects(
    effects: &[EffectAst],
    found: &mut Vec<(PlayerAst, PlayerAst, Option<PlayerFilter>)>,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    chooser,
                    player,
                    ..
                },
            ..
        }) = effect
        {
            found.push((*chooser, *player, filter.owner.clone()));
        }
        for_each_nested_effects(effect, true, |nested| search_subjects(nested, found));
    }
}

#[test]
fn targeted_search_instead_branch_carries_owner_without_changing_implicit_chooser() {
    let lexed = crate::lexer::lex_line(
            "Search target player's library for up to three cards, exile them, then that player shuffles. If this spell was kicked, instead search that player's library for up to fifteen cards, exile them, then that player shuffles.",
            0,
        )
        .expect("targeted search self-replacement should lex");
    let parsed = parse_effect_sentences_lexed(&lexed)
        .expect("targeted search self-replacement should parse");
    let (if_true, if_false) = parsed
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SelfReplacement {
                if_true, if_false, ..
            } => Some((if_true, if_false)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a targeted-search self-replacement: {parsed:#?}"));

    let mut subjects = Vec::new();
    search_subjects(if_false, &mut subjects);
    search_subjects(if_true, &mut subjects);
    assert_eq!(
        subjects,
        vec![
            (
                PlayerAst::Implicit,
                PlayerAst::That,
                Some(PlayerFilter::target_player()),
            ),
            (
                PlayerAst::Implicit,
                PlayerAst::That,
                Some(PlayerFilter::target_player()),
            ),
        ],
        "both branches carry the target-qualified library while preserving a demonstrative action surface"
    );

    let lowered = crate::compile_support::compile_statement_effects_with_imports(
        &parsed,
        &crate::model::reference_state::ReferenceImports::default(),
    )
    .expect("targeted search self-replacement should lower");
    let debug = format!("{lowered:#?}");
    assert!(!debug.contains("IteratedPlayer"), "{debug}");
    assert_eq!(debug.matches("chooser: You").count(), 2, "{debug}");
}
