use super::*;
use crate::lexer::lex_line;

#[test]
fn additional_copy_count_keeps_typed_authored_surface() {
    let raw = [
        "Choose target instant or sorcery spell.",
        "Each opponent may copy that spell and may choose new targets for the copy they control.",
        "You copy that spell once plus an additional time for each opponent who copied the spell this way.",
        "You may choose new targets for the copies you control.",
    ];
    let lexed: Vec<Vec<OwnedLexToken>> = raw
        .iter()
        .enumerate()
        .map(|(index, text)| lex_line(text, index).expect("tempting-offer sentence should lex"))
        .collect();
    let sentences: Vec<SentenceInput> = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect();
    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("tempting-offer parser should not error")
        .expect("tempting-offer copy sequence should match");
    assert!(matches!(
        effects.first(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly {
                explicit_declaration: true,
                ..
            },
            ..
        }))
    ));
    assert!(matches!(
            effects.last(),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CopySpell {
                    count_surface: Some(
                        ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay
                    ),
                    ..
                },
                ..
            }))
        ));
}
