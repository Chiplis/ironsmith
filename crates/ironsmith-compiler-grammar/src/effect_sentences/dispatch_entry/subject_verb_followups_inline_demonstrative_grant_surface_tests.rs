use super::*;

#[test]
fn rebuilding_a_demonstrative_grant_keeps_its_those_surface() {
    let tokens = crate::lexer::lex_line("Those creatures gain vigilance until end of turn.", 0)
        .expect("demonstrative grant should lex");
    let antecedent = ObjectFilter::creature().you_control().other();
    let rebuilt = build_grant_all_from_demonstrative_gain(antecedent.clone(), &tokens)
        .expect("demonstrative grant should parse")
        .expect("demonstrative grant should rebuild against its antecedent");

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                set_quantifier_surface,
                ..
            },
        ..
    }) = rebuilt
    else {
        panic!("expected a filter-wide grant");
    };
    assert_eq!(filter, antecedent);
    assert_eq!(
        set_quantifier_surface,
        Some(ironsmith_core::SetQuantifierSurface::Those)
    );
}
