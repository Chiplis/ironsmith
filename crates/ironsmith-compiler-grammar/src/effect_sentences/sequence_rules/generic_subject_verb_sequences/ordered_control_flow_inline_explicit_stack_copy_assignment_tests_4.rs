use super::*;
use crate::lexer::lex_line;

fn parse(second: &str) -> Option<Vec<EffectAst>> {
    let lexed = [
            lex_line(
                "Choose target instant or sorcery spell that targets only a single permanent or player.",
                0,
            )
            .unwrap(),
            lex_line(second, 1).unwrap(),
            lex_line(
                "Each copy targets a different one of those permanents and players.",
                2,
            )
            .unwrap(),
        ];
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();
    crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .unwrap()
}

#[test]
fn announced_stack_target_and_copy_share_the_unresolved_reference_tag() {
    let tokens = lex_line(
            "Choose target instant or sorcery spell that targets only a single permanent or player. Copy that spell for each other permanent or player the spell could target. Each copy targets a different one of those permanents and players.",
            0,
        )
        .unwrap();
    let effects = effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("public declared-target copy assignment");
    assert!(matches!(
        effects.as_slice(),
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget {
                    target: TargetAst::Tagged(tag, _),
                    exclude_current_targets: true,
                    ..
                }),
                ..
            })
        ] if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    ));
}

#[test]
fn source_spell_reference_does_not_claim_the_declared_target_shape() {
    assert!(
        parse("Copy this spell for each other permanent or player the spell could target.")
            .is_none()
    );
}
